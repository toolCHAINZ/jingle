use std::fmt::Display;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::analysis::cfg::CfgState;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::residue::VecReducer;
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState, RunnableConfigurableProgramAnalysis};
use crate::analysis::linkage::PcodeReverseLinkage;
use crate::analysis::pcode_store::PcodeStore;

pub mod state;

pub use state::ReverseLocationState;

/// A CPA that traverses the CFG in reverse, following backward edges.
///
/// Wraps a [`PcodeReverseLinkage<N>`] and provides predecessor-based transfer:
/// for each node, `transfer` yields all forward-CFG predecessors, enabling
/// backward dataflow analyses (e.g. liveness) when composed with a suitable
/// secondary CPA.
pub struct ReverseLocationAnalysis<N: CfgState, L: PcodeReverseLinkage<N>> {
    linkage: Arc<L>,
    _phantom: PhantomData<N>,
}

impl<N: CfgState, L: PcodeReverseLinkage<N>> ReverseLocationAnalysis<N, L> {
    /// Construct a reverse analysis from any [`PcodeReverseLinkage`] implementor.
    #[must_use]
    pub fn new(linkage: Arc<L>) -> Self {
        Self {
            linkage,
            _phantom: PhantomData,
        }
    }

    /// Return a reference to the underlying linkage.
    #[must_use]
    pub fn linkage(&self) -> &L {
        &self.linkage
    }
}

impl<N, L> ConfigurableProgramAnalysis for ReverseLocationAnalysis<N, L>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + 'static,
    L: PcodeReverseLinkage<N> + 'static,
{
    type State = ReverseLocationState<N, L>;
    type Reducer<'op> = VecReducer;
}

impl<N, L> IntoState<ReverseLocationAnalysis<N, L>> for N
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + 'static,
    L: PcodeReverseLinkage<N> + 'static,
{
    fn into_state(self, c: &ReverseLocationAnalysis<N, L>) -> ReverseLocationState<N, L> {
        ReverseLocationState {
            inner: self,
            linkage: Arc::clone(&c.linkage),
        }
    }
}

impl<N, L> ReverseLocationAnalysis<N, L>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + 'static,
    L: PcodeReverseLinkage<N> + 'static,
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
    ) -> Vec<ReverseLocationState<N, L>>
    where
        ReverseLocationState<N, L>: 'op,
    {
        let mut all = Vec::new();
        for leaf in self.linkage.leaf_nodes() {
            let initial = ReverseLocationState {
                inner: leaf,
                linkage: Arc::clone(&self.linkage),
            };
            let results = self.run_cpa(initial, store);
            all.extend(results);
        }
        all
    }
}
