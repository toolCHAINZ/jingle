use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::analysis::cfg::CfgState;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::residue::liveness_map::LivenessMapReducer;
use crate::analysis::cpa::{ConfigurableProgramAnalysis, RunnableConfigurableProgramAnalysis};
use crate::analysis::linkage::PcodeReverseLinkage;
use crate::analysis::liveness::cpa_state::LivenessCpaState;
use crate::analysis::pcode_store::PcodeStore;

pub mod annotated;
pub mod cpa_state;
pub mod state;

pub use annotated::LivenessAnnotated;
pub use state::LivenessState;

/// Self-contained backward liveness analysis.
///
/// Wraps a [`PcodeReverseLinkage<N>`] and drives the classic union-based
/// liveness computation:
///
/// ```text
/// live_in(node) = gen(node) ∪ (live_out(node) − kill(node))
/// ```
///
/// Use [`LivenessAnalysis::run_from_leaves`] to obtain a
/// `HashMap<N, LivenessState>` mapping every CFG node to its live-in set.
pub struct LivenessAnalysis<N: CfgState, L: PcodeReverseLinkage<N>> {
    linkage: Arc<L>,
    _phantom: PhantomData<N>,
}

impl<N, L> LivenessAnalysis<N, L>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + Hash + Eq + 'static,
    L: PcodeReverseLinkage<N> + 'static,
{
    /// Construct a liveness analysis from any [`PcodeReverseLinkage`] implementor.
    #[must_use]
    pub fn new(linkage: Arc<L>) -> Self {
        Self {
            linkage,
            _phantom: PhantomData,
        }
    }

    /// Run backward liveness analysis from every leaf node, returning
    /// `live_in` sets keyed by CFG node.
    ///
    /// When multiple leaves can reach the same node, their liveness
    /// contributions are joined (union).
    pub fn run_from_leaves<'op, T: PcodeStore<'op> + ?Sized>(
        &self,
        store: &'op T,
    ) -> HashMap<N, LivenessState>
    where
        LivenessCpaState<N, L>: 'op,
    {
        let mut result: HashMap<N, LivenessState> = HashMap::new();
        for leaf in self.linkage.leaf_nodes() {
            let initial = LivenessCpaState {
                location: leaf,
                live: LivenessState::empty(),
                linkage: Arc::clone(&self.linkage),
            };
            let partial: HashMap<N, LivenessState> = self.run_cpa(initial, store);
            for (node, liveness) in partial {
                result
                    .entry(node)
                    .and_modify(|e| e.join(&liveness))
                    .or_insert(liveness);
            }
        }
        result
    }
}

impl<N, L> ConfigurableProgramAnalysis for LivenessAnalysis<N, L>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + Hash + Eq + 'static,
    L: PcodeReverseLinkage<N> + 'static,
{
    type State = LivenessCpaState<N, L>;
    type Reducer<'op> = LivenessMapReducer<N, L>;
}
