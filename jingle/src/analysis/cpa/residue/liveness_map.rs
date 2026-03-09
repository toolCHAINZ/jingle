use std::collections::HashMap;
use std::marker::PhantomData;

use crate::analysis::cfg::CfgState;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::residue::Residue;
use crate::analysis::linkage::PcodeReverseLinkage;
use crate::analysis::liveness::cpa_state::LivenessCpaState;
use crate::analysis::liveness::state::LivenessState;

/// Reducer that folds all reached [`LivenessCpaState`] values into a
/// `HashMap` keyed by the CFG node, with liveness sets joined across
/// multiple paths to the same node.
pub struct LivenessMapReducer<N, L>(PhantomData<(N, L)>);

impl<'op, N, L> Residue<'op, LivenessCpaState<N, L>> for LivenessMapReducer<N, L>
where
    N: CfgState + std::hash::Hash + Eq,
    L: PcodeReverseLinkage<N>,
{
    type Output = HashMap<N, LivenessState>;

    fn new() -> Self {
        LivenessMapReducer(PhantomData)
    }

    fn finalize(self, reached: Vec<LivenessCpaState<N, L>>) -> HashMap<N, LivenessState> {
        let mut map = HashMap::new();
        for state in reached {
            map.entry(state.location)
                .and_modify(|e: &mut LivenessState| e.join(&state.live))
                .or_insert(state.live);
        }
        map
    }
}
