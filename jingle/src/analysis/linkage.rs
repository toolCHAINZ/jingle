use itertools::Itertools;
use petgraph::Direction;
use petgraph::graph::NodeIndex;
use petgraph::prelude::StableDiGraph;
use petgraph::visit::EdgeRef;
use std::collections::HashMap;

use crate::analysis::cfg::{CfgNode, EmptyEdge, PcodeCfg};

/// Provides forward CFG traversal: successors, entries, and leaves.
pub trait PcodeLinkage<N: CfgNode> {
    fn successors_of(&self, node: &N) -> Vec<N>;
    fn entry_nodes(&self) -> Vec<N>;
    fn leaf_nodes_fwd(&self) -> Vec<N>;
    fn all_nodes(&self) -> Vec<N>;
}

/// Provides backward CFG traversal: predecessors and backward starting points.
pub trait PcodeReverseLinkage<N: CfgNode> {
    fn predecessors_of(&self, node: &N) -> Vec<N>;
    /// Leaf nodes of the forward CFG — starting points for backward analysis.
    fn leaf_nodes(&self) -> Vec<N>;
}

impl<N: CfgNode, D: Clone> PcodeReverseLinkage<N> for PcodeCfg<N, D> {
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

impl<N: CfgNode, D: Clone> PcodeLinkage<N> for PcodeCfg<N, D> {
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

impl<N: CfgNode> PcodeLinkage<N> for Vec<N> {
    fn all_nodes(&self) -> Vec<N> {
        self.clone()
    }

    fn leaf_nodes_fwd(&self) -> Vec<N> {
        vec![self.last().unwrap().clone()]
    }

    fn entry_nodes(&self) -> Vec<N> {
        vec![self[0].clone()]
    }

    fn successors_of(&self, node: &N) -> Vec<N> {
        let idx = self.iter().find_position(|p| *p == node);
        if let Some((i, _)) = idx {
            if i + 1 < self.len() {
                vec![self[i + 1].clone()]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }
}

/// Lightweight linkage built from a CFG's graph structure alone.
///
/// Captures the graph topology without the op map (`D`), so it is `'static`
/// whenever `N: 'static`, regardless of the op storage type of the source CFG.
pub struct CfgLinkage<N: CfgNode> {
    graph: StableDiGraph<N, EmptyEdge>,
    indices: HashMap<N, NodeIndex>,
}

impl<N: CfgNode> CfgLinkage<N> {
    pub fn from_cfg<D>(cfg: &PcodeCfg<N, D>) -> Self {
        Self {
            graph: cfg.graph.clone(),
            indices: cfg.indices.clone(),
        }
    }
}

impl<N: CfgNode> PcodeReverseLinkage<N> for CfgLinkage<N> {
    fn predecessors_of(&self, node: &N) -> Vec<N> {
        let Some(&idx) = self.indices.get(node) else {
            return vec![];
        };
        self.graph
            .edges_directed(idx, Direction::Incoming)
            .map(|e| self.graph.node_weight(e.source()).unwrap().clone())
            .collect()
    }

    fn leaf_nodes(&self) -> Vec<N> {
        self.graph
            .externals(Direction::Outgoing)
            .map(|idx| self.graph.node_weight(idx).unwrap().clone())
            .collect()
    }
}

impl<N: CfgNode> PcodeReverseLinkage<N> for Vec<N> {
    fn predecessors_of(&self, node: &N) -> Vec<N> {
        let idx = self.iter().find_position(|p| *p == node);
        if let Some((i, _)) = idx {
            if i > 0 {
                vec![self[i - 1].clone()]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }

    fn leaf_nodes(&self) -> Vec<N> {
        vec![self[0].clone()]
    }
}
