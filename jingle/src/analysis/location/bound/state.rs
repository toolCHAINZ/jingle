use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::analysis::location::bound::FallthroughCounting;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::{Ordering, Reverse};
use std::fmt::Display;
use std::iter::{empty, once};

/// A simple analysis counting the number of branches on a path,
/// terminating when it hits the max
///
/// todo: refactor and extend this type to allow for bounding different
/// things: program transitions, jumps, states matching a predicate?
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct BoundedBranchState {
    pub branch_count: usize,
    max_count: usize,
    /// How to treat Fallthrough pcode operations when counting branches.
    pub fallthrough_counting: FallthroughCounting,
}

impl BoundedBranchState {
    pub fn new(max_count: usize, fallthrough_counting: FallthroughCounting) -> Self {
        Self {
            max_count,
            branch_count: 0,
            fallthrough_counting,
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
        self.branch_count = self.branch_count.max(other.branch_count);
    }
}

impl AbstractState for BoundedBranchState {
    fn merge(&mut self, other: &Self) -> MergeOutcome<Self> {
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
            // Determine whether this opcode should be counted as a branch.
            let is_branch = opcode.branch_destination().is_some();
            let is_fallthrough = matches!(opcode, PcodeOperation::Fallthrough { .. });
            let should_count = is_branch
                && !(matches!(self.fallthrough_counting, FallthroughCounting::Ignore)
                    && is_fallthrough);

            let cur = if should_count {
                self.branch_count + 1
            } else {
                self.branch_count
            };
            let max_count = self.max_count;
            let branch_count = cur;
            let fallthrough_counting = self.fallthrough_counting;
            once(Self {
                max_count,
                branch_count,
                fallthrough_counting,
            })
            .into()
        }
    }
}

impl Display for BoundedBranchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.branch_count)
    }
}
