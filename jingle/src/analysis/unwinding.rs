use crate::JingleError;
use crate::analysis::Analysis;
use crate::analysis::cfg::{CfgState, CfgStateModel, ModelTransition, PcodeCfg};
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::simple::SimpleLattice;
use crate::analysis::cpa::lattice::{JoinSemiLattice, PartialJoinSemiLattice};
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::analysis::pcode_store::PcodeStore;
use crate::analysis::unwinding::UnwoundLocation::{Location, UnwindError};
use crate::modeling::machine::MachineState;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashMap;
use z3::ast::Bool;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum UnwoundLocation {
    UnwindError(ConcretePcodeAddress),
    Location(usize, ConcretePcodeAddress),
}

#[derive(Debug, Clone, Eq)]
pub struct UnwindingCpaState {
    location: ConcretePcodeAddress,
    visits: HashMap<ConcretePcodeAddress, usize>,
    max: usize,
}

impl UnwindingCpaState {
    pub fn new(location: ConcretePcodeAddress, max: usize) -> Self {
        let mut s = UnwindingCpaState {
            location,
            visits: Default::default(),
            max,
        };
        s.increment_visit_count();
        s
    }

    pub fn location(&self) -> ConcretePcodeAddress {
        self.location
    }

    pub fn visit_count(&self) -> usize {
        *self.visits.get(&self.location).unwrap_or(&0)
    }

    pub fn increment_visit_count(&mut self) {
        let new = self.visit_count() + 1;
        self.visits.insert(self.location, new);
    }
}

impl PartialEq for UnwindingCpaState {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location
    }
}
impl PartialOrd for UnwindingCpaState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.location() == other.location() {
            let self_visit = self.visits.get(&self.location).unwrap_or(&0);
            let other_visit = other.visits.get(&self.location).unwrap_or(&0);
            self_visit.partial_cmp(other_visit)
        } else {
            None
        }
    }
}

impl PartialJoinSemiLattice for UnwindingCpaState {
    fn partial_join(&self, other: &Self) -> Option<Self> {
        if self.location == other.location {
            let mut visits = HashMap::new();
            for (addr, count) in self.visits.iter() {
                let count = *count;
                let max: usize = count.max(other.visits.get(addr).cloned().unwrap_or(0));
                visits.insert(*addr, max);
            }
            let s = Self {
                location: self.location,
                visits,
                max: self.max,
            };
            Some(s)
        } else {
            None
        }
    }
}

impl JoinSemiLattice for UnwindingCpaState {
    fn join(&mut self, other: &Self) {
        if self.location == other.location {
            for (addr, count) in self.visits.iter_mut() {
                let max: usize = other.visits.get(addr).cloned().unwrap_or(0);
                *count = max;
            }
        }
    }
}

impl AbstractState for UnwindingCpaState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }
    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, opcode: B) -> Successor<'a, Self> {
        if self.visit_count() > self.max {
            return std::iter::empty().into();
        }
        self.location
            .transfer(opcode.borrow())
            .into_iter()
            .map(|location| {
                let visits = self.visits.clone();
                let mut next = Self {
                    location,
                    visits,
                    max: self.max,
                };
                next.increment_visit_count();
                next
            })
            .into()
    }
}

impl LocationState for UnwindingCpaState {
    fn get_operation<T: PcodeStore>(&self, t: &T) -> Option<PcodeOperation> {
        t.get_pcode_op_at(self.location)
    }
}

impl UnwoundLocation {
    pub fn location(&self) -> &ConcretePcodeAddress {
        match self {
            UnwindError(a) => a,
            Location(_, a) => a,
        }
    }

    pub fn new(loc: &ConcretePcodeAddress, count: &usize) -> Self {
        Self::Location(*count, *loc)
    }

    pub fn is_unwind_error(&self) -> bool {
        matches!(self, UnwindError(_))
    }

    pub fn from_cpa_state(a: &UnwindingCpaState, max: usize) -> Self {
        if a.visit_count() - 1 > max {
            UnwindError(a.location())
        } else {
            Location(a.visit_count(), a.location())
        }
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
        let pc = self.state.pc().eq(other.state.pc());
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

pub type UnwoundCfg = PcodeCfg<UnwoundLocation, PcodeOperation>;

impl ModelTransition<UnwoundLocation> for PcodeOperation {
    fn transition(&self, init: &UnwoundLocationModel) -> Result<UnwoundLocationModel, JingleError> {
        Ok(UnwoundLocationModel {
            is_unwind_error: Bool::fresh_const("u"),
            state: init.state.apply(self)?,
        })
    }
}
struct UnwoundLocationCPA<T: PcodeStore> {
    source_cfg: T,
    unwound_cfg: PcodeCfg<UnwoundLocation, PcodeOperation>,
}

impl<T: PcodeStore> ConfigurableProgramAnalysis for UnwoundLocationCPA<T> {
    type State = SimpleLattice<UnwindingCpaState>;

    fn get_pcode_store(&self) -> &impl PcodeStore {
        &self.source_cfg
    }

    fn reduce(&mut self, state: &Self::State, dest_state: &Self::State) {
        if let SimpleLattice::Value(a) = state {
            let a = UnwoundLocation::from_cpa_state(a, a.max);
            self.unwound_cfg.add_node(a);
            if !a.is_unwind_error() {
                if let Some(op) = self.source_cfg.get_pcode_op_at(a.location()) {
                    let dest = UnwoundLocation::from_cpa_state(
                        dest_state.value().unwrap(),
                        dest_state.value().unwrap().max,
                    );
                    self.unwound_cfg.add_edge(a, dest, op)
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
        let mut cpa = UnwoundLocationCPA {
            source_cfg: store,
            unwound_cfg: Default::default(),
        };
        let init_state = UnwindingCpaState::new(addr, self.max);
        let _ = cpa.run_cpa(&SimpleLattice::Value(init_state));
        cpa.unwound_cfg
    }

    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input {
        addr
    }
}
