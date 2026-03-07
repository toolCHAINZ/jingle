use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use jingle_sleigh::PcodeOperation;

use crate::analysis::cfg::{CfgState, PcodeCfg};
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::analysis::pcode_store::{PcodeOpRef, PcodeStore};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

/// Abstract state for reverse CFG traversal.
///
/// Wraps a forward-analysis node `N` and holds a shared reference to the
/// forward `PcodeCfg` so that `transfer` can look up predecessors.
///
/// `PartialEq`, `Eq`, and `Hash` delegate to `inner` only — the Arc pointer is
/// not part of identity; all states in a single run share the same CFG.
pub struct ReverseLocationState<N: CfgState> {
    pub(super) inner: N,
    pub(super) cfg: Arc<PcodeCfg<N, PcodeOperation>>,
}

impl<N: CfgState> PartialEq for ReverseLocationState<N> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<N: CfgState> Eq for ReverseLocationState<N> {}

impl<N: CfgState> Hash for ReverseLocationState<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<N: CfgState> Clone for ReverseLocationState<N> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            cfg: Arc::clone(&self.cfg),
        }
    }
}

impl<N: CfgState + PartialOrd> PartialOrd for ReverseLocationState<N> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<N: CfgState> Debug for ReverseLocationState<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReverseLocationState")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<N: CfgState + Display> Display for ReverseLocationState<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rev({})", self.inner)
    }
}

impl<N: CfgState + JoinSemiLattice> JoinSemiLattice for ReverseLocationState<N> {
    fn join(&mut self, other: &Self) {
        self.inner.join(&other.inner);
    }
}

impl<N> AbstractState for ReverseLocationState<N>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + 'static,
{
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    /// Return all CFG predecessors of the current node as successor states.
    ///
    /// The `op` argument (the operation at the current node) is unused — predecessor
    /// discovery comes from the CFG Arc, not from the operation itself.
    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, _op: B) -> Successor<'a, Self> {
        let preds: Arc<Vec<N>> = Arc::new(
            self.cfg
                .predecessors(&self.inner)
                .unwrap_or_default()
                .into_iter()
                .cloned()
                .collect(),
        );
        let n = preds.len();
        let cfg = Arc::clone(&self.cfg);
        (0..n)
            .map(move |i| ReverseLocationState {
                inner: preds[i].clone(),
                cfg: Arc::clone(&cfg),
            })
            .into()
    }
}

impl<N> LocationState for ReverseLocationState<N>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + 'static,
{
    fn get_operation<'op, T: PcodeStore<'op> + ?Sized>(&self, t: &'op T) -> Option<PcodeOpRef<'op>> {
        let addr = self.inner.concrete_location()?;
        t.get_pcode_op_at(addr)
    }

    fn get_location(&self) -> Option<ConcretePcodeAddress> {
        self.inner.concrete_location()
    }
}
