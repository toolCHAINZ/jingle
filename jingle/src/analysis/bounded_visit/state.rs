use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::iter::empty;

#[derive(Eq, Clone, Debug)]
pub struct BoundedStepsState {
    pub location: PcodeAddressLattice,
    pub branch_count: usize,
    max_count: usize,
}

impl BoundedStepsState {
    pub fn new(location: PcodeAddressLattice, max_count: usize) -> Self {
        Self {
            location,
            max_count,
            branch_count: 0,
        }
    }
}

impl From<ConcretePcodeAddress> for BoundedStepsState {
    fn from(addr: ConcretePcodeAddress) -> Self {
        // Default max_count - this is a problem since we need max_count from somewhere
        // For now, use a large default
        Self::new(PcodeAddressLattice::Value(addr), 1000)
    }
}

impl PartialEq<Self> for BoundedStepsState {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location && self.branch_count == other.branch_count
    }
}

impl PartialOrd for BoundedStepsState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.location.partial_cmp(&other.location) {
            Some(Ordering::Equal) => other.branch_count.partial_cmp(&self.branch_count),
            Some(o) => {
                if other.branch_count.cmp(&self.branch_count) == o {
                    Some(o)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl JoinSemiLattice for BoundedStepsState {
    fn join(&mut self, other: &Self) {
        self.branch_count = self.branch_count.min(other.branch_count);
        self.location.join(&other.location);
    }
}

impl AbstractState for BoundedStepsState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        if self.location == other.location {
            self.merge_join(other)
        } else {
            self.merge_sep(other)
        }
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, opcode: B) -> Successor<'a, Self> {
        let opcode = opcode.borrow();
        if self.branch_count == self.max_count {
            empty().into()
        } else {
            let cur = if opcode.branch_destination().is_some() {
                self.branch_count + 1
            } else {
                self.branch_count
            };
            let max_count = self.max_count;
            self.location
                .transfer(opcode)
                .into_iter()
                .map(move |location| Self {
                    location,
                    branch_count: cur,
                    max_count,
                })
                .into()
        }
    }
}

impl LocationState for BoundedStepsState {
    fn get_operation<T: PcodeStore>(&self, t: &T) -> Option<PcodeOperation> {
        self.location.get_operation(t)
    }
}
