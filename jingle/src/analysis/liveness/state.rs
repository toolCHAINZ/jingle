use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use jingle_sleigh::{GeneralizedVarNode, PcodeOperation, VarNode};

use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::analysis::varnode::VarNodeSet;

/// Abstract state for liveness analysis.
///
/// Tracks the set of varnodes that are *live* at a given program point: a
/// varnode is live if it may be read before it is next written on some path
/// from the current point.  Union-based (may-live) semantics are used, so
/// the live set is over-approximated conservatively.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LivenessState {
    live: VarNodeSet,
}

impl LivenessState {
    /// Create an initial liveness state with no live varnodes (empty live set).
    ///
    /// This is the correct initial state for leaf nodes in a backward analysis:
    /// nothing is live past the end of the program.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            live: VarNodeSet::default(),
        }
    }

    /// Create a liveness state with a pre-populated live set.
    #[must_use]
    pub fn with_set(live: VarNodeSet) -> Self {
        Self { live }
    }

    /// Iterate over the varnodes currently in the live set.
    pub fn live_varnodes(&self) -> impl Iterator<Item = VarNode> + '_ {
        self.live.varnodes()
    }

    /// Returns `true` if `vn` is live at this program point.
    ///
    /// Uses `VarNodeSet::covers` which handles overlapping ranges correctly.
    #[must_use]
    pub fn is_live(&self, vn: &VarNode) -> bool {
        self.live.covers(vn)
    }

    /// Compute `live_in = reads(op) ∪ (live_out − kill(op))` — the backward transfer.
    ///
    /// Returns the liveness state on entry to `op`, given `self` as the liveness
    /// state on exit. Used by [`super::cpa_state::LivenessCpaState::transfer`].
    pub(crate) fn apply_transfer(&self, op: &PcodeOperation) -> Self {
        let (reads, kill) = reads_kill(op);
        let mut new_live = self.live.clone();
        new_live.subtract(&kill);
        new_live.union(&reads);
        Self { live: new_live }
    }
}

/// Compute the reads and kill sets for a single pcode operation.
///
/// - **reads**: varnodes that are read by the operation (direct inputs) plus the
///   pointer varnode for each indirect (memory) input.
/// - **kill**: the single direct output varnode, if any.  Indirect outputs
///   (memory stores) conservatively produce no kill so we over-approximate
///   the live set.
pub(crate) fn reads_kill(op: &PcodeOperation) -> (VarNodeSet, VarNodeSet) {
    let mut reads = VarNodeSet::default();
    let mut kill = VarNodeSet::default();
    if matches!(
        op,
        PcodeOperation::Branch { .. } | PcodeOperation::Fallthrough { .. }
    ) {
        return (reads, kill);
    }

    match op {
        PcodeOperation::Branch { .. } | PcodeOperation::Fallthrough { .. } => {
            return (reads, kill);
        }
        PcodeOperation::CBranch { input1, .. } => {
            reads.insert(&input1);
            return (reads, kill);
        }
        PcodeOperation::Call {
            args, call_info, ..
        } => {
            for arg in args {
                reads.insert(arg);
            }
            if let Some(call_info) = call_info {
                for ele in &call_info.killed_regs {
                    kill.insert(ele);
                }
            }
            return (reads, kill);
        }
        _ => {}
    }

    for input in op
        .inputs()
        .iter()
        .filter(|i| i.space_index() != VarNode::CONST_SPACE_INDEX as usize)
    {
        match input {
            GeneralizedVarNode::Direct(vn) => reads.insert(&vn),
            GeneralizedVarNode::Indirect(ivn) => reads.insert(ivn.pointer_location()),
        }
    }

    if let Some(GeneralizedVarNode::Direct(vn)) = op.output() {
        kill.insert(&vn);
    }

    (reads, kill)
}

impl Hash for LivenessState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut vns: Vec<VarNode> = self.live.varnodes().collect();
        vns.sort();
        vns.hash(state);
    }
}

impl Display for LivenessState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let count = self.live.varnodes().count();
        write!(f, "Live({count} vars)")
    }
}

impl PartialOrd for LivenessState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.live.partial_cmp(&other.live)
    }
}

impl JoinSemiLattice for LivenessState {
    /// Join via union: the may-live set at a merge point is the union of all
    /// incoming live sets.
    fn join(&mut self, other: &Self) {
        self.live.union(&other.live);
    }
}

impl AbstractState for LivenessState {
    /// Merge at join points using union (`merge_join`).
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_join(other)
    }

    /// Stop when the current live set is already a subset of some reached state.
    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    /// Transfer function: `live_in = reads(op) ∪ (live_out − kill(op))`.
    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, op: B) -> Successor<'a, Self> {
        let (reads, kill) = reads_kill(op.borrow());
        let mut new_live = self.live.clone();
        new_live.subtract(&kill);
        new_live.union(&reads);
        std::iter::once(Self { live: new_live }).into()
    }
}
