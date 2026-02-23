pub mod state;

use crate::analysis::cpa::residue::EmptyResidue;
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::location::bound::state::BoundedBranchState;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

/// How to treat instruction fallthrough pcode operations when counting branches.
///
/// - `Ignore` (default): do not count `PcodeOperation::Fallthrough` as a branch.
/// - `Count`: count fallthroughs as branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FallthroughCounting {
    Ignore,
    Count,
}

/// Analysis that bounds the number of observed branches/transitions.
///
/// The constructor `new` defaults to ignoring fallthrough pcode operations
/// (so by default we count ISA instructions rather than ISA basic blocks).
pub struct BoundedBranchAnalysis {
    max_steps: usize,
    fallthrough_counting: FallthroughCounting,
}

impl BoundedBranchAnalysis {
    /// Create a new analysis that ignores fallthroughs by default.
    pub fn new(max_steps: usize) -> Self {
        Self {
            max_steps,
            fallthrough_counting: FallthroughCounting::Ignore,
        }
    }

    /// Create a new analysis with an explicit fallthrough counting mode.
    pub fn with_fallthrough_counting(max_steps: usize, mode: FallthroughCounting) -> Self {
        Self {
            max_steps,
            fallthrough_counting: mode,
        }
    }

    /// Convenience constructor that preserves the old behaviour: count all branches,
    /// including fallthroughs.
    pub fn new_counting_all(max_steps: usize) -> Self {
        Self::with_fallthrough_counting(max_steps, FallthroughCounting::Count)
    }

    /// Access the configured fallthrough counting mode.
    pub fn fallthrough_counting(&self) -> FallthroughCounting {
        self.fallthrough_counting
    }

    /// Access the configured maximum number of steps/branches.
    pub fn max_steps(&self) -> usize {
        self.max_steps
    }
}

impl ConfigurableProgramAnalysis for BoundedBranchAnalysis {
    type State = BoundedBranchState;
    type Reducer<'op> = EmptyResidue<Self::State>;
}

impl IntoState<BoundedBranchAnalysis> for ConcretePcodeAddress {
    fn into_state(self, c: &BoundedBranchAnalysis) -> BoundedBranchState {
        // Pass the configured mode into the initial state.
        BoundedBranchState::new(c.max_steps, c.fallthrough_counting)
    }
}
