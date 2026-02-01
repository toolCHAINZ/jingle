use crate::analysis::{
    cpa::{ConfigurableProgramAnalysis, residue::EmptyResidue},
    location::unwind::state::UnwindingState,
};

pub(crate) mod state;

/// Internal CPA for back-edge counting
pub struct UnwindingAnalysis {
    max_count: usize,
}

impl UnwindingAnalysis {
    pub fn new(max_count: usize) -> Self {
        Self { max_count }
    }
}

impl ConfigurableProgramAnalysis for UnwindingAnalysis {
    type State = UnwindingState;
    type Reducer<'op> = EmptyResidue<UnwindingState>;
}
