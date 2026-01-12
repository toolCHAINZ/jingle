mod state;

use crate::analysis::Analysis;
use crate::analysis::bounded_branch::state::BoundedBranchState;
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

pub struct BoundedBranchAnalysis {
    max_steps: usize,
}

impl BoundedBranchAnalysis {
    pub fn new(max_steps: usize) -> Self {
        Self { max_steps }
    }
}

impl ConfigurableProgramAnalysis for BoundedBranchAnalysis {
    type State = BoundedBranchState;
}

impl IntoState<BoundedBranchAnalysis> for ConcretePcodeAddress {
    fn into_state(self, c: &BoundedBranchAnalysis) -> BoundedBranchState {
        BoundedBranchState::new(c.max_steps)
    }
}

impl Analysis for BoundedBranchAnalysis {}
