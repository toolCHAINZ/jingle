use crate::analysis::cfg::{ModelTransition, PcodeCfg};
use crate::analysis::cpa::lattice::flat::FlatLattice::Value;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::lattice::simple::SimpleLattice;
use crate::analysis::cpa::lattice::{JoinSemiLattice, PartialJoinSemiLattice};
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::analysis::unwinding::UnwoundLocation::{Location, UnwindError};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::io::empty;
use crate::analysis::pcode_store::PcodeStore;

#[derive(Eq, PartialEq, Copy, Clone)]
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
                _ => None,
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


struct UnwoundLocationCPA<T: PcodeStore>{
    cfg: T,

}