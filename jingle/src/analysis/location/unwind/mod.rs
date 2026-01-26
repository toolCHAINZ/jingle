use crate::analysis::{
    cpa::{ConfigurableProgramAnalysis, residue::EmptyResidue},
    location::unwind::state::BackEdgeCountState,
};

mod state;

/// Internal CPA for back-edge counting
pub struct BackEdgeCountCPA {
    max_count: usize,
}

impl BackEdgeCountCPA {
    pub fn new(max_count: usize) -> Self {
        Self { max_count }
    }
}

impl ConfigurableProgramAnalysis for BackEdgeCountCPA {
    type State = BackEdgeCountState;
    type Reducer = EmptyResidue<BackEdgeCountState>;
}
