pub mod state;

use crate::analysis::cpa::residue::EmptyResidue;
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::location::bound::state::BoundedBranchState;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

/// Analysis that bounds the number of observed instructions and/or branches.
///
/// This analysis supports three primary configurations:
/// - Bound only the number of branches (backwards-compatible with the old API).
/// - Bound only the number of instructions.
/// - Bound both instructions and branches simultaneously.
///
/// Use the provided constructors to choose the desired configuration.
pub struct BoundedBranchAnalysis {
    /// Maximum number of ISA instructions.
    max_instructions: Option<usize>,
    /// Maximum number of explicit ISA branches.
    max_branches: Option<usize>,
    /// Maximum number of pcode steps
    max_ops: Option<usize>,
}

impl BoundedBranchAnalysis {
    /// Backwards-compatible constructor: bounds the number of branches (old behaviour).
    pub fn new(max_branches: usize) -> Self {
        Self {
            max_instructions: None,
            max_branches: Some(max_branches),
            max_ops: None,
        }
    }

    /// Create an analysis that bounds only instructions (ignoring branch counts).
    pub fn new_instruction_bound(max_instructions: usize) -> Self {
        Self {
            max_instructions: Some(max_instructions),
            max_branches: None,
            max_ops: None,
        }
    }

    /// Create an analysis with explicit optional bounds for instructions and branches.
    /// Use `None` for any bound you do not want to apply.
    pub fn with_bounds(
        max_instructions: Option<usize>,
        max_branches: Option<usize>,
        max_ops: Option<usize>,
    ) -> Self {
        Self {
            max_instructions,
            max_branches,
            max_ops,
        }
    }

    /// Access the optional configured maximum number of branches.
    pub fn max_branches(&self) -> Option<usize> {
        self.max_branches
    }

    /// Access the optional configured maximum number of instructions.
    pub fn max_instructions(&self) -> Option<usize> {
        self.max_instructions
    }

    pub fn max_ops(&self) -> Option<usize> {
        self.max_ops
    }
}

impl ConfigurableProgramAnalysis for BoundedBranchAnalysis {
    type State = BoundedBranchState;
    type Reducer<'op> = EmptyResidue<Self::State>;
}

impl IntoState<BoundedBranchAnalysis> for ConcretePcodeAddress {
    fn into_state(self, c: &BoundedBranchAnalysis) -> BoundedBranchState {
        BoundedBranchState::with_all_bounds(c.max_instructions, c.max_branches, c.max_ops)
    }
}
