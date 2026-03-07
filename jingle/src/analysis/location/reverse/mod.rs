use std::fmt::Display;
use std::sync::Arc;

use jingle_sleigh::PcodeOperation;

use crate::analysis::cfg::{CfgState, PcodeCfg};
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::residue::VecReducer;
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState, RunnableConfigurableProgramAnalysis};
use crate::analysis::pcode_store::PcodeStore;

pub mod state;

pub use state::ReverseLocationState;

/// A CPA that traverses the CFG in reverse, following backward edges.
///
/// Wraps a pre-built [`PcodeCfg`] and provides predecessor-based transfer:
/// for each node, `transfer` yields all forward-CFG predecessors, enabling
/// backward dataflow analyses (e.g. liveness) when composed with a suitable
/// secondary CPA.
pub struct ReverseLocationAnalysis<N: CfgState> {
    cfg: Arc<PcodeCfg<N, PcodeOperation>>,
}

impl<N: CfgState> ReverseLocationAnalysis<N> {
    /// Construct a reverse analysis from an existing forward CFG.
    #[must_use]
    pub fn new(cfg: Arc<PcodeCfg<N, PcodeOperation>>) -> Self {
        Self { cfg }
    }

    /// Return a reference to the underlying forward CFG.
    #[must_use]
    pub fn cfg(&self) -> &PcodeCfg<N, PcodeOperation> {
        &self.cfg
    }
}

impl<N> ConfigurableProgramAnalysis for ReverseLocationAnalysis<N>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + 'static,
{
    type State = ReverseLocationState<N>;
    type Reducer<'op> = VecReducer;
}

impl<N> IntoState<ReverseLocationAnalysis<N>> for N
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + 'static,
{
    fn into_state(self, c: &ReverseLocationAnalysis<N>) -> ReverseLocationState<N> {
        ReverseLocationState {
            inner: self,
            cfg: Arc::clone(&c.cfg),
        }
    }
}

impl<N> ReverseLocationAnalysis<N>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + 'static,
{
    /// Run the reverse analysis from every leaf node of the forward CFG,
    /// collecting all reached states into a single `Vec`.
    ///
    /// A leaf node is one with no forward successors (i.e., a program exit).
    /// Starting from each leaf, the CPA walks backward until a fixed point is
    /// reached for each leaf's reachable subgraph.
    #[must_use]
    pub fn run_from_leaves<'op, T: PcodeStore<'op> + ?Sized>(
        &self,
        store: &'op T,
    ) -> Vec<ReverseLocationState<N>> {
        let mut all = Vec::new();
        for leaf in self.cfg.leaf_nodes() {
            let initial = ReverseLocationState {
                inner: leaf.clone(),
                cfg: Arc::clone(&self.cfg),
            };
            let results = self.run_cpa(initial, store);
            all.extend(results);
        }
        all
    }
}
