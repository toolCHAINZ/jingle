use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::residue::VecReducer;

pub mod state;

pub use state::LivenessState;

/// CPA component for liveness analysis.
///
/// `LivenessAnalysis` is a zero-sized struct that pairs with
/// [`crate::analysis::location::reverse::ReverseLocationAnalysis`] via the
/// tuple compound-CPA pattern to compute classic union-based liveness:
///
/// ```text
/// live_in(node) = gen(node) ∪ (live_out(node) − kill(node))
/// ```
///
/// Run the compound `(ReverseLocationAnalysis, LivenessAnalysis)` from every
/// CFG leaf node (with an empty initial live set) to obtain liveness
/// information at every program point.
pub struct LivenessAnalysis;

impl ConfigurableProgramAnalysis for LivenessAnalysis {
    type State = LivenessState;
    type Reducer<'op> = VecReducer;
}
