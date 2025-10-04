use crate::JingleError;
use crate::analysis::back_edge::{BackEdgeAnalysis, BackEdges};
use crate::analysis::cfg::{CfgState, CfgStateModel, ModelTransition, PcodeCfg};
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::PartialJoinSemiLattice;
use crate::analysis::cpa::lattice::flat::FlatLattice::Value;
use crate::analysis::cpa::lattice::simple::SimpleLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::analysis::pcode_store::PcodeStore;
use crate::analysis::unwinding::UnwoundLocation::{Location, UnwindError};
use crate::analysis::{Analysis, AnalyzableBase};
use crate::modeling::machine::MachineState;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use std::borrow::Borrow;
use std::cmp::Ordering;
use z3::ast::Bool;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum UnwoundLocation {
    UnwindError(ConcretePcodeAddress),
    Location(usize, ConcretePcodeAddress),
}

impl UnwoundLocation {
    pub fn location(&self) -> &ConcretePcodeAddress {
        match self {
            UnwindError(a) => a,
            Location(_, a) => a,
        }
    }

    pub fn new(loc: &ConcretePcodeAddress, count: &usize) -> Self {
        Self::Location(count.clone(), loc.clone())
    }

    pub fn is_unwind_error(&self) -> bool {
        matches!(self, UnwindError(_))
    }
}

#[derive(Debug, Clone)]
pub struct UnwoundLocationModel {
    is_unwind_error: Bool,
    state: MachineState,
}

impl CfgStateModel for UnwoundLocationModel {
    fn location_eq(&self, other: &Self) -> Bool {
        let unwind = self.is_unwind_error.eq(&other.is_unwind_error);
        let pc = self.state.pc().eq(&other.state.pc());
        unwind & pc
    }

    fn eq(&self, other: &Self) -> Bool {
        self.is_unwind_error.eq(&other.is_unwind_error) & self.state.eq(&other.state)
    }
}
impl CfgState for UnwoundLocation {
    type Model = UnwoundLocationModel;

    fn fresh(&self, i: &SleighArchInfo) -> Self::Model {
        let state = MachineState::fresh(i);
        UnwoundLocationModel {
            state,
            is_unwind_error: Bool::from_bool(self.is_unwind_error()),
        }
    }
}

type UnwoundLocationLattice = SimpleLattice<UnwoundLocation>;

pub type UnwoundCfg = PcodeCfg<UnwoundLocation, PcodeOperation>;

impl ModelTransition<UnwoundLocation> for PcodeOperation {
    fn transition(&self, init: &UnwoundLocationModel) -> Result<UnwoundLocationModel, JingleError> {
        Ok(UnwoundLocationModel {
            is_unwind_error: Bool::fresh_const("u"),
            state: init.state.apply(self)?,
        })
    }
}
impl PartialOrd for UnwoundLocation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_loc = self.location();
        let other_loc = other.location();
        if self_loc == other_loc {
            match (self, other) {
                (UnwindError(_), UnwindError(_)) => Some(Ordering::Equal),
                (Location(a_count, ..), Location(b_count, ..)) => a_count.partial_cmp(&b_count),
                (UnwindError(_), Location(..)) => Some(Ordering::Greater),
                (Location(..), UnwindError(_)) => Some(Ordering::Less),
            }
        } else {
            None
        }
    }
}

impl PartialJoinSemiLattice for UnwoundLocation {
    fn partial_join(&self, other: &Self) -> Option<Self> {
        if self.location() == other.location() {
            if self >= other {
                Some(self.clone())
            } else {
                Some(other.clone())
            }
        } else {
            None
        }
    }
}

impl AbstractState for UnwoundLocationLattice {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        // using merge_sep because we don't actually need to merge states for
        // duplicate visits; the duplicate will never get added to the waitlist because of stop
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, opcode: B) -> Successor<'a, Self> {
        match self {
            UnwoundLocationLattice::Value(Location(count, loc)) => loc
                .transfer(opcode.borrow())
                .into_iter()
                .flat_map(|l| Some(SimpleLattice::Value(UnwoundLocation::new(&l, count))))
                .into(),
            _ => std::iter::empty().into(),
        }
    }
}

struct UnwoundLocationCPA<T: PcodeStore> {
    source_cfg: T,
    max: usize,
    back_edges: BackEdges,
    unwound_cfg: PcodeCfg<UnwoundLocation, PcodeOperation>,
}

impl<T: PcodeStore> ConfigurableProgramAnalysis for UnwoundLocationCPA<T> {
    type State = UnwoundLocationLattice;

    fn successor_states<'a>(&self, state: &'a Self::State) -> Successor<'a, Self::State> {
        if let Some(Location(count, loc)) = state.value()
            && let Some(op) = self.source_cfg.get_pcode_op_at(loc)
        {
            if count >= &self.max {
                return std::iter::empty().into();
            }
            let edges = self.back_edges.get_all_for(loc);
            state
                .transfer(op)
                .into_iter()
                .map(move |a| {
                    if let SimpleLattice::Value(Location(count, dest_loc)) = a
                        && edges
                            .as_ref()
                            .map(|a| a.contains(&dest_loc))
                            .unwrap_or(false)
                    {
                        SimpleLattice::Value(Location(count + 1, dest_loc))
                    } else {
                        a
                    }
                })
                .into()
        } else {
            std::iter::empty().into()
        }
    }

    fn reduce(&mut self, state: &Self::State, dest_state: &Self::State) {
        if let SimpleLattice::Value(a) = state {
            self.unwound_cfg.add_node(a);
            if !a.is_unwind_error() {
                if let Some(op) = self.source_cfg.get_pcode_op_at(a.location()) {
                    self.unwound_cfg
                        .add_edge(a, dest_state.value().unwrap(), op)
                }
            }
        }
    }
}

pub struct UnwindingAnalysis {
    max: usize,
}

impl UnwindingAnalysis {
    pub fn new(max: usize) -> Self {
        Self { max }
    }
}
impl Analysis for UnwindingAnalysis {
    type Output = PcodeCfg<UnwoundLocation, PcodeOperation>;
    type Input = ConcretePcodeAddress;

    fn run<T: PcodeStore, I: Into<Self::Input>>(
        &mut self,
        store: T,
        initial_state: I,
    ) -> Self::Output {
        let addr = initial_state.into();
        let back_edges = store.run_analysis_at(addr, BackEdgeAnalysis);
        let mut cpa = UnwoundLocationCPA {
            back_edges,
            max: self.max,
            source_cfg: store,
            unwound_cfg: Default::default(),
        };
        let init_state = UnwoundLocation::Location(0, addr);
        let _ = cpa.run_cpa(&SimpleLattice::Value(init_state));
        cpa.unwound_cfg
    }

    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input {
        addr
    }
}
