use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use jingle_sleigh::{SleighArchInfo, VarNode};

use crate::analysis::cfg::{CfgState, PcodeCfg};
use crate::analysis::linkage::CfgLinkage;
use crate::display::JingleDisplay;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::state::PcodeLocation;
use crate::analysis::liveness::{LivenessAnalysis, LivenessState};
use crate::analysis::pcode_store::PcodeStore;

/// A CFG node decorated with the liveness state computed at that program point.
///
/// `live_in` holds the set of varnodes live on entry to the node's operation,
/// as computed by a backward union-based liveness analysis.
///
/// Identity (`PartialEq`, `Eq`, `Hash`) is based solely on `node`; the live
/// set is auxiliary metadata and does not affect equality or hashing.
#[derive(Clone, Debug)]
pub struct LivenessAnnotated<N: CfgState> {
    pub node: N,
    pub live_in: LivenessState,
}

impl<N: CfgState> PartialEq for LivenessAnnotated<N> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl<N: CfgState> Eq for LivenessAnnotated<N> {}

impl<N: CfgState> Hash for LivenessAnnotated<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}

impl<N: CfgState + PartialOrd> PartialOrd for LivenessAnnotated<N> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.node.partial_cmp(&other.node)
    }
}

impl<N: CfgState + Display> Display for LivenessAnnotated<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.node, f)
    }
}

impl<N: CfgState> PcodeLocation for LivenessAnnotated<N> {
    fn location(&self) -> PcodeAddressLattice {
        self.node.location()
    }
}

impl<N: CfgState + JingleDisplay> JingleDisplay for LivenessAnnotated<N> {
    fn fmt_jingle(&self, f: &mut std::fmt::Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        self.node.fmt_jingle(f, info)?;
        let mut live: Vec<VarNode> = self.live_in.live_varnodes().collect();
        live.sort();
        write!(f, "  live: [")?;
        let mut first = true;
        for vn in &live {
            if !first {
                write!(f, ", ")?;
            }
            vn.fmt_jingle(f, info)?;
            first = false;
        }
        write!(f, "]")
    }
}

impl<N: CfgState> CfgState for LivenessAnnotated<N> {
    type Model = N::Model;

    fn new_const(&self, i: &SleighArchInfo) -> N::Model {
        self.node.new_const(i)
    }

    fn model_id(&self) -> String {
        self.node.model_id()
    }
}

impl<N, D: Clone> PcodeCfg<N, D>
where
    N: CfgState + JoinSemiLattice + Display + PartialOrd + std::hash::Hash + Eq + 'static,
{
    /// Enrich this CFG with liveness annotations computed by backward analysis.
    ///
    /// Returns a new `PcodeCfg` with the same graph structure but with each
    /// node replaced by a [`LivenessAnnotated`] wrapper carrying the liveness
    /// state at that program point.
    ///
    /// `store` is the [`PcodeStore`] used to look up p-code operations during
    /// the backward traversal; passing `self` works for the common case where
    /// the forward CFG is also the instruction source.
    pub fn annotate_liveness<'op, T: PcodeStore<'op> + ?Sized>(
        &self,
        store: &'op T,
    ) -> PcodeCfg<LivenessAnnotated<N>, D> {
        let liveness_map =
            LivenessAnalysis::new(Arc::new(CfgLinkage::from_cfg(self))).run_from_leaves(store);

        let annotated_node = |n: &N| LivenessAnnotated {
            node: n.clone(),
            live_in: liveness_map
                .get(n)
                .cloned()
                .unwrap_or_else(LivenessState::empty),
        };

        let mut enriched: PcodeCfg<LivenessAnnotated<N>, D> = PcodeCfg::new();

        // Add all nodes first to capture leaves that have no outgoing edges.
        for node in self.nodes() {
            enriched.add_node(annotated_node(node));
        }

        // Add edges (which also registers ops keyed by the `from` address).
        for from in self.nodes() {
            if let (Some(succs), Some(op)) = (self.successors(from), self.get_op_at(from)) {
                for to in succs {
                    enriched.add_edge(annotated_node(from), annotated_node(to), op.clone());
                }
            }
        }

        enriched
    }
}
