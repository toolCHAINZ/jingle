mod state;

use crate::analysis::Analysis;
use crate::analysis::bounded_visit::state::BoundedBranchState;
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

struct BoundedBranchCpa {
    max_steps: usize,
}

impl BoundedBranchCpa {
    pub fn new(max_steps: usize) -> Self {
        Self { max_steps }
    }
}

impl ConfigurableProgramAnalysis for BoundedBranchCpa {
    type State = BoundedBranchState;
}

impl IntoState<BoundedBranchCpa> for ConcretePcodeAddress {
    fn into_state(self, c: &BoundedBranchCpa) -> BoundedBranchState {
        todo!()
    }
}

impl Analysis for BoundedBranchCpa {}

pub type BoundedBranchAnalysis = BoundedBranchCpa;
