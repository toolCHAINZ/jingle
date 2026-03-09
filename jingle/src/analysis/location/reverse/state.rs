use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use jingle_sleigh::PcodeOperation;

use crate::analysis::cfg::CfgState;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::state::{
    AbstractState, LocationState, MergeOutcome, PcodeLocation, Successor,
};
use crate::analysis::linkage::PcodeReverseLinkage;
use crate::analysis::pcode_store::{PcodeOpRef, PcodeStore};

/// Abstract state for reverse CFG traversal.
///
/// Wraps a forward-analysis node `N` and holds a shared reference to a
/// [`PcodeReverseLinkage<N>`] so that `transfer` can look up predecessors.
///
/// `PartialEq`, `Eq`, and `Hash` delegate to `inner` only — the Arc pointer is
/// not part of identity; all states in a single run share the same linkage.
pub struct ReverseLocationState<N: CfgState, L: PcodeReverseLinkage<N>> {
    pub(super) inner: N,
    pub(super) linkage: Arc<L>,
}

impl<N: CfgState, L: PcodeReverseLinkage<N>> ReverseLocationState<N, L> {
    /// Returns a reference to the wrapped CFG node.
    pub fn node(&self) -> &N {
        &self.inner
    }
}

impl<N: CfgState, L: PcodeReverseLinkage<N>> PartialEq for ReverseLocationState<N, L> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<N: CfgState, L: PcodeReverseLinkage<N>> Eq for ReverseLocationState<N, L> {}

impl<N: CfgState, L: PcodeReverseLinkage<N>> Hash for ReverseLocationState<N, L> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<N: CfgState, L: PcodeReverseLinkage<N>> Clone for ReverseLocationState<N, L> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            linkage: Arc::clone(&self.linkage),
        }
    }
}

impl<N: CfgState + PartialOrd, L: PcodeReverseLinkage<N>> PartialOrd
    for ReverseLocationState<N, L>
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<N: CfgState, L: PcodeReverseLinkage<N>> Debug for ReverseLocationState<N, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReverseLocationState")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<N: CfgState + Display, L: PcodeReverseLinkage<N>> Display for ReverseLocationState<N, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rev({})", self.inner)
    }
}

impl<N: CfgState + JoinSemiLattice, L: PcodeReverseLinkage<N>> JoinSemiLattice
    for ReverseLocationState<N, L>
{
    fn join(&mut self, other: &Self) {
        self.inner.join(&other.inner);
    }
}

impl<N, L> AbstractState for ReverseLocationState<N, L>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + 'static,
    L: PcodeReverseLinkage<N> + 'static,
{
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    /// Return all CFG predecessors of the current node as successor states.
    ///
    /// The `op` argument (the operation at the current node) is unused —
    /// predecessor discovery comes from the linkage, not from the operation.
    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, _op: B) -> Successor<'a, Self> {
        let preds: Arc<Vec<N>> = Arc::new(self.linkage.predecessors_of(&self.inner));
        let n = preds.len();
        let linkage = Arc::clone(&self.linkage);
        (0..n)
            .map(move |i| ReverseLocationState {
                inner: preds[i].clone(),
                linkage: Arc::clone(&linkage),
            })
            .into()
    }
}

impl<N: CfgState, L: PcodeReverseLinkage<N>> PcodeLocation for ReverseLocationState<N, L> {
    fn location(&self) -> crate::analysis::cpa::lattice::pcode::PcodeAddressLattice {
        self.inner.location()
    }
}

impl<N, L> LocationState for ReverseLocationState<N, L>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + 'static,
    L: PcodeReverseLinkage<N> + 'static,
{
    fn get_operation<'op, T: PcodeStore<'op> + ?Sized>(
        &self,
        t: &'op T,
    ) -> Option<PcodeOpRef<'op>> {
        let addr = self.inner.concrete_location()?;
        t.get_pcode_op_at(addr)
    }
}
