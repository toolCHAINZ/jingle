use crate::analysis::back_edge::BackEdges;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::PartialJoinSemiLattice;
use crate::analysis::cpa::lattice::simple::SimpleLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::analysis::pcode_store::PcodeStore;
use crate::analysis::unwinding::UnwoundLocation::{Location, UnwindError};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
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
}

type UnwoundLocationLattice = SimpleLattice<UnwoundLocation>;

pub type UnwoundCfg = PcodeCfg<UnwoundLocation, PcodeOperation>;

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
    cfg: T,
    max: usize,
    back_edges: BackEdges,
}

impl<T: PcodeStore> ConfigurableProgramAnalysis for UnwoundLocationCPA<T> {
    type State = UnwoundLocationLattice;

    fn successor_states<'a>(&self, state: &'a Self::State) -> Successor<'a, Self::State> {
        if let Some(Location(count, loc)) = state.value()
            && let Some(op) = self.cfg.get_pcode_op_at(loc)
        {
            if count >= &self.max {
                return std::iter::empty().into();
            }
            let o = self.back_edges.clone();
            state
                .transfer(op)
                .into_iter()
                .map(move |a| {
                    if let SimpleLattice::Value(Location(count, dest_loc)) = a
                        && o.has(loc, &dest_loc)
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
}
