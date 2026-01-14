use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::{Ordering, Reverse};
use std::iter::{empty, once};

/// A simple analysis counting the number of branches on a path,
/// terminating when it hits the max
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct BoundedBranchState {
    pub branch_count: usize,
    max_count: usize,
}

impl BoundedBranchState {
    pub fn new(max_count: usize) -> Self {
        Self {
            max_count,
            branch_count: 0,
        }
    }
}

impl PartialOrd for BoundedBranchState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // reversing the order here because we are measuring the minimum steps to get
        // somewhere: a lower number is shorter, so it is _greater_ in the lattice
        Reverse(self.branch_count).partial_cmp(&Reverse(other.branch_count))
    }
}

impl JoinSemiLattice for BoundedBranchState {
    fn join(&mut self, other: &Self) {
        self.branch_count = self.branch_count.min(other.branch_count);
    }
}

impl AbstractState for BoundedBranchState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_join(other)
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
            let branch_count = cur;
            once(Self {
                max_count,
                branch_count,
            })
            .into()
        }
    }
}
