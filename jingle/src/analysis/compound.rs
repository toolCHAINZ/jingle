use crate::analysis::cpa::ConfigurableProgramAnalysis;

pub enum StrengthenOutcome {
    Changed,
    Unchanged,
}
trait CompoundAnalysis<O: ConfigurableProgramAnalysis>: ConfigurableProgramAnalysis {
    fn sharpen(&self, left: &mut Self::State, other: &O::State) -> StrengthenOutcome;
}

