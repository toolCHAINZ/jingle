use crate::analysis::cfg::model::ModelTransition;
use crate::analysis::cfg::{CfgState, PcodeCfg};

/// Provides forward CFG traversal: successors, entries, and leaves.
pub trait PcodeLinkage<N: CfgState> {
    fn successors_of(&self, node: &N) -> Vec<N>;
    fn entry_nodes(&self) -> Vec<N>;
    fn leaf_nodes_fwd(&self) -> Vec<N>;
    fn all_nodes(&self) -> Vec<N>;
}

/// Provides backward CFG traversal: predecessors and backward starting points.
pub trait PcodeReverseLinkage<N: CfgState> {
    fn predecessors_of(&self, node: &N) -> Vec<N>;
    /// Leaf nodes of the forward CFG — starting points for backward analysis.
    fn leaf_nodes(&self) -> Vec<N>;
}

impl<N: CfgState, D: ModelTransition<N::Model> + Clone> PcodeReverseLinkage<N> for PcodeCfg<N, D> {
    fn predecessors_of(&self, node: &N) -> Vec<N> {
        self.predecessors(node)
            .unwrap_or_default()
            .into_iter()
            .cloned()
            .collect()
    }

    fn leaf_nodes(&self) -> Vec<N> {
        // Use the inherent PcodeCfg::leaf_nodes via explicit type-qualified syntax
        // to avoid infinite recursion with the trait method of the same name.
        PcodeCfg::leaf_nodes(self).cloned().collect()
    }
}

impl<N: CfgState, D: ModelTransition<N::Model> + Clone> PcodeLinkage<N> for PcodeCfg<N, D> {
    fn successors_of(&self, node: &N) -> Vec<N> {
        self.successors(node)
            .unwrap_or_default()
            .into_iter()
            .cloned()
            .collect()
    }

    fn entry_nodes(&self) -> Vec<N> {
        PcodeCfg::entry_nodes(self).cloned().collect()
    }

    fn leaf_nodes_fwd(&self) -> Vec<N> {
        PcodeCfg::leaf_nodes(self).cloned().collect()
    }

    fn all_nodes(&self) -> Vec<N> {
        self.nodes().cloned().collect()
    }
}
