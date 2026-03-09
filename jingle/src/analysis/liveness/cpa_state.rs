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
use crate::analysis::liveness::state::LivenessState;
use crate::analysis::pcode_store::{PcodeOpRef, PcodeStore};

/// Combined backward-traversal CPA state for liveness analysis.
///
/// Bundles a CFG location (`N`) with its live varnode set and a shared
/// reference to the backward linkage. This eliminates the need for an
/// external `(ReverseLocationAnalysis, LivenessAnalysis)` compound tuple.
///
/// **Identity** (`PartialEq`, `Eq`, `Hash`) is based on `location` only so
/// that states at the same program point are merged by the CPA algorithm
/// regardless of their current live set.
///
/// **Ordering** (`PartialOrd`) is incomparable for different locations;
/// for the same location it delegates to `live.partial_cmp` (subset order).
/// This makes `stop_sep` correctly stop when liveness is already covered.
pub struct LivenessCpaState<N: CfgState, L: PcodeReverseLinkage<N>> {
    pub(crate) location: N,
    pub(crate) live: LivenessState,
    pub(crate) linkage: Arc<L>,
}

// --- Identity: location only ---

impl<N: CfgState, L: PcodeReverseLinkage<N>> PartialEq for LivenessCpaState<N, L> {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location
    }
}

impl<N: CfgState, L: PcodeReverseLinkage<N>> Eq for LivenessCpaState<N, L> {}

impl<N: CfgState, L: PcodeReverseLinkage<N>> Hash for LivenessCpaState<N, L> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.location.hash(state);
    }
}

// --- Ordering: live-based at the same location, incomparable otherwise ---
//
// Note: This is intentionally inconsistent with `PartialEq` (which uses only
// `location`). The `PartialOrd` is used exclusively by `stop_sep` to check
// lattice subsumption; `PartialEq` is used for CPA merge identity. The two
// concepts are deliberately separated here.
impl<N: CfgState + PartialOrd, L: PcodeReverseLinkage<N>> PartialOrd for LivenessCpaState<N, L> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.location != other.location {
            return None;
        }
        self.live.partial_cmp(&other.live)
    }
}

// --- Clone: avoid L: Clone bound ---

impl<N: CfgState, L: PcodeReverseLinkage<N>> Clone for LivenessCpaState<N, L> {
    fn clone(&self) -> Self {
        Self {
            location: self.location.clone(),
            live: self.live.clone(),
            linkage: Arc::clone(&self.linkage),
        }
    }
}

// --- Debug / Display: avoid L: Debug / L: Display bounds ---

impl<N: CfgState, L: PcodeReverseLinkage<N>> Debug for LivenessCpaState<N, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LivenessCpaState")
            .field("location", &self.location)
            .field("live", &self.live)
            .finish()
    }
}

impl<N: CfgState + Display, L: PcodeReverseLinkage<N>> Display for LivenessCpaState<N, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Liveness({}, {})", self.location, self.live)
    }
}

// --- JoinSemiLattice: join on live set ---

impl<N: CfgState + PartialOrd, L: PcodeReverseLinkage<N>> JoinSemiLattice
    for LivenessCpaState<N, L>
{
    fn join(&mut self, other: &Self) {
        self.live.join(&other.live);
    }
}

// --- AbstractState ---

impl<N, L> AbstractState for LivenessCpaState<N, L>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + 'static,
    L: PcodeReverseLinkage<N> + 'static,
{
    /// Merge liveness when locations match; no-op for different locations.
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        if self.location != other.location {
            return MergeOutcome::NoOp;
        }
        let old_live = self.live.clone();
        self.live.join(&other.live);
        if self.live == old_live {
            MergeOutcome::NoOp
        } else {
            MergeOutcome::Merged
        }
    }

    /// Stop when `self` is already subsumed by a reached state at the same location.
    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    /// Backward transfer: compute `live_in = reads(op) ∪ (live_out − kill(op))`,
    /// then yield one successor per CFG predecessor of the current location.
    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, op: B) -> Successor<'a, Self> {
        let new_live = self.live.apply_transfer(op.borrow());
        let preds = self.linkage.predecessors_of(&self.location);
        let linkage = Arc::clone(&self.linkage);
        preds
            .into_iter()
            .map(move |pred| LivenessCpaState {
                location: pred,
                live: new_live.clone(),
                linkage: Arc::clone(&linkage),
            })
            .into()
    }
}

// --- PcodeLocation ---

impl<N: CfgState, L: PcodeReverseLinkage<N>> PcodeLocation for LivenessCpaState<N, L> {
    fn location(&self) -> crate::analysis::cpa::lattice::pcode::PcodeAddressLattice {
        self.location.location()
    }
}

// --- LocationState ---

impl<N, L> LocationState for LivenessCpaState<N, L>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + 'static,
    L: PcodeReverseLinkage<N> + 'static,
{
    fn get_operation<'op, T: PcodeStore<'op> + ?Sized>(
        &self,
        t: &'op T,
    ) -> Option<PcodeOpRef<'op>> {
        let addr = self.location.concrete_location()?;
        t.get_pcode_op_at(addr)
    }
}
