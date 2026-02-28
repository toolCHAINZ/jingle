use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::{Ordering, Reverse};
use std::fmt::Display;
use std::iter::{empty, once};

/// A state that can bound either:
/// - the number of ISA instructions visited (`max_instructions`),
/// - the number of branches visited (`max_branches`),
/// - or both.
///
/// This variant always ignores fallthrough operations when counting branches.
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct BoundedBranchState {
    /// Number of instructions visited so far.
    pub instruction_count: usize,
    /// Number of counted branches visited so far.
    pub branch_count: usize,

    /// Optional maximum number of instructions to allow.
    pub max_instructions: Option<usize>,
    /// Optional maximum number of branches to allow.
    pub max_branches: Option<usize>,
}

impl BoundedBranchState {
    /// Backwards-compatible constructor: treat `max_count` as the maximum number
    /// of branches (same as the previous `BoundedBranchState::new`).
    pub fn new(max_count: usize) -> Self {
        Self {
            instruction_count: 0,
            branch_count: 0,
            max_instructions: None,
            max_branches: Some(max_count),
        }
    }

    /// Create a state that bounds only the number of instructions visited.
    pub fn with_instruction_bound(max_instructions: usize) -> Self {
        Self {
            instruction_count: 0,
            branch_count: 0,
            max_instructions: Some(max_instructions),
            max_branches: None,
        }
    }

    /// Create a state with explicit optional bounds for instructions and branches.
    /// Use `None` for any bound you do not want to apply.
    pub fn with_both_bounds(max_instructions: Option<usize>, max_branches: Option<usize>) -> Self {
        Self {
            instruction_count: 0,
            branch_count: 0,
            max_instructions,
            max_branches,
        }
    }

    /// Check whether either configured bound has been reached.
    fn is_at_bound(&self) -> bool {
        if let Some(max_i) = self.max_instructions {
            if self.instruction_count >= max_i {
                return true;
            }
        }
        if let Some(max_b) = self.max_branches {
            if self.branch_count >= max_b {
                return true;
            }
        }
        false
    }
}

impl PartialOrd for BoundedBranchState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // We treat "smaller" counts as better (shorter paths). Reverse the tuple
        // so that lower instruction/branch counts compare as "greater" in the lattice.
        // We compare instruction count first, then branch count as a tie-breaker.
        Reverse((self.instruction_count, self.branch_count))
            .partial_cmp(&Reverse((other.instruction_count, other.branch_count)))
    }
}

impl JoinSemiLattice for BoundedBranchState {
    fn join(&mut self, other: &Self) {
        // For our lattice ordering (where smaller counts are considered "greater"),
        // the join must produce an element that is >= both operands. Given that
        // ordering, we must choose the minimum of the counts so the joined state
        // does not become \"worse\" (i.e., have larger counts) than its components.
        self.instruction_count = self.instruction_count.min(other.instruction_count);
        self.branch_count = self.branch_count.min(other.branch_count);

        // Merge configured bounds permissively: if either side is unbounded (None),
        // the joined state should be unbounded (None). If both sides have a bound,
        // choose the more permissive bound (the larger numeric limit) so we don't
        // accidentally discard successors that one side would allow.
        self.max_instructions = match (self.max_instructions, other.max_instructions) {
            (Some(a), Some(b)) => Some(a.max(b)),
            _ => None,
        };

        self.max_branches = match (self.max_branches, other.max_branches) {
            (Some(a), Some(b)) => Some(a.max(b)),
            _ => None,
        };
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

        // If we've already hit any configured bound, stop exploring.
        if self.is_at_bound() {
            return empty().into();
        }

        // Determine whether this opcode should be counted as a branch.
        // Always ignore fallthrough operations when counting branches.
        let is_branch = opcode.branch_destination().is_some();
        let is_fallthrough = matches!(opcode, PcodeOperation::Fallthrough { .. });
        let should_count_branch = is_branch && !is_fallthrough;

        let next_instruction_count = self.instruction_count + 1;
        let next_branch_count = if should_count_branch {
            self.branch_count + 1
        } else {
            self.branch_count
        };

        // If the successor would exceed any configured bound, do not produce it.
        if match self.max_instructions {
            Some(max) => next_instruction_count > max,
            None => false,
        } || match self.max_branches {
            Some(max) => next_branch_count > max,
            None => false,
        } {
            return empty().into();
        }

        let next = Self {
            instruction_count: next_instruction_count,
            branch_count: next_branch_count,
            max_instructions: self.max_instructions,
            max_branches: self.max_branches,
        };

        once(next).into()
    }
}

impl Display for BoundedBranchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.max_instructions, self.max_branches) {
            (Some(mi), Some(mb)) => {
                write!(
                    f,
                    "i:{}/{} b:{}/{}",
                    self.instruction_count, mi, self.branch_count, mb
                )
            }
            (Some(mi), None) => write!(
                f,
                "i:{}/{} b:{}",
                self.instruction_count, mi, self.branch_count
            ),
            (None, Some(mb)) => write!(
                f,
                "i:{} b:{}/{}",
                self.instruction_count, self.branch_count, mb
            ),
            (None, None) => write!(f, "i:{} b:{}", self.instruction_count, self.branch_count),
        }
    }
}
