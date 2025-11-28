use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
pub use model::{CfgState, CfgStateModel, ModelTransition};
use petgraph::Direction;
use petgraph::graph::NodeIndex;
use petgraph::prelude::DiGraph;
use petgraph::visit::EdgeRef;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{Formatter, LowerHex};
use z3::ast::Bool;
use z3::{Params, Solver};
use crate::analysis::ctl::CtlFormula;
use crate::analysis::unwinding::{UnwoundLocation, UnwoundLocationModel};
use crate::JingleError;

mod model;

#[derive(Debug, Default, Copy, Clone, Hash)]
pub struct EmptyEdge;

impl LowerHex for EmptyEdge {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

#[derive(Debug)]
pub struct PcodeCfg<N: CfgState = ConcretePcodeAddress, D = PcodeOperation> {
    pub(crate) graph: DiGraph<N, EmptyEdge>,
    pub(crate) ops: HashMap<N, D>,
    pub(crate) indices: HashMap<N, NodeIndex>,
    pub(crate) models: HashMap<N, N::Model>,
}

impl<N: CfgState,D> Default for PcodeCfg<N, D>{
    fn default() -> Self {
        Self{
            graph: Default::default(),
            ops: Default::default(),
            indices: Default::default(),
            models: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct PcodeCfgVisitor<'a, N: CfgState, D>{
    cfg: &'a PcodeCfg<N,D>,
    location: N
}

impl<'a, N: CfgState, D:  ModelTransition<N::Model>> PcodeCfgVisitor<'a, N, D>{
    pub(crate) fn successors(&self) -> impl Iterator<Item=Self>{
        self.cfg.successors(&self.location).into_iter().flatten().map(|n|Self{
            cfg: self.cfg,
            location: n.clone()
        })
    }

    pub(crate) fn transition(&self) -> Option<&D>{
        self.cfg.ops.get(&self.location)
    }

    pub(crate) fn location(&self) -> &N{
        &self.location
    }
    pub(crate) fn state(&self) -> Option<&N::Model>{
        self.cfg.models.get(&self.location)
    }
}


pub struct PcodeCfgView<'a, N: CfgState = ConcretePcodeAddress, D = PcodeOperation> {
    /// Borrowed reference to the original CFG (zero-copy view)
    pub cfg: &'a PcodeCfg<N, D>,
    /// The node index that is the entry/origin for this view
    pub origin: NodeIndex,
    /// Set of node indices reachable from `origin` (inclusive)
    pub nodes: std::collections::HashSet<NodeIndex>,
}

impl<'a, N: CfgState, D> PcodeCfgView<'a, N, D> {
    /// Returns a reference to the origin node's weight
    pub fn origin_node(&self) -> &N {
        self.cfg
            .graph
            .node_weight(self.origin)
            .expect("origin node index should be valid")
    }

    /// Returns the nodes included in this view as a Vec of references into the backing CFG
    pub fn nodes(&self) -> Vec<&N> {
        self.nodes
            .iter()
            .map(|idx| self.cfg.graph.node_weight(*idx).unwrap())
            .collect()
    }

    /// Get successors of a node within this view (only those that are part of the view's reachable set)
    pub fn successors_of(&self, node: &N) -> Option<Vec<&N>>
    where
        N: std::hash::Hash + Eq,
    {
        let idx = self.cfg.indices.get(node)?;
        if !self.nodes.contains(idx) {
            return None;
        }
        let succs: Vec<&N> = self
            .cfg
            .graph
            .edges_directed(*idx, Direction::Outgoing)
            .filter_map(|e| {
                let w = self.cfg.graph.node_weight(e.target()).unwrap();
                // only include successors inside the view
                let tidx = self.cfg.indices.get(w).unwrap();
                if self.nodes.contains(tidx) {
                    Some(w)
                } else {
                    None
                }
            })
            .collect();
        Some(succs)
    }
}

impl<N: CfgState, D: ModelTransition<N::Model>> PcodeCfg<N, D> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn graph(&self) -> &DiGraph<N, EmptyEdge> {
        &self.graph
    }

    pub fn nodes(&self) -> impl Iterator<Item = &N> {
        self.indices.keys()
    }

    /// Check whether the CFG contains a node (by value)
    pub fn has_node<T: Borrow<N>>(&self, node: T) -> bool {
        self.indices.contains_key(node.borrow())
    }

    pub fn get_op_at<T: Borrow<N>>(&self, addr: T) -> Option<&D> {
        self.ops.get(addr.borrow())
    }

    pub fn add_node<T: Borrow<N>>(&mut self, node: T) {
        let node = node.borrow();
        if !self.indices.contains_key(node) {
            let idx = self.graph.add_node(node.clone());
            self.indices.insert(node.clone(), idx);
        }
    }

    pub fn add_edge<A, B, C>(&mut self, from: A, to: B, op: C)
    where
        A: Borrow<N>,
        B: Borrow<N>,
        C: Borrow<D>,
    {
        let from = from.borrow();
        let to = to.borrow();
        let op = op.borrow();
        self.add_node(from);
        self.add_node(to);
        self.ops.insert(from.clone(), op.clone());
        let from_idx = *self.indices.get(from).unwrap();
        let to_idx = *self.indices.get(to).unwrap();
        // Return early if the edge already exists
        if self.graph.find_edge(from_idx, to_idx).is_some() {
            return;
        }
        self.graph.add_edge(from_idx, to_idx, EmptyEdge);
    }

    /// Return successors of a node (by value) as references into the backing CFG.
    /// Returns `None` if the node is not present in the CFG.
    pub fn successors<T: Borrow<N>>(&self, node: T) -> Option<Vec<&N>> {
        let n = node.borrow();
        let idx = *self.indices.get(n)?;
        let succs: Vec<&N> = self
            .graph
            .edges_directed(idx, Direction::Outgoing)
            .map(|e| self.graph.node_weight(e.target()).unwrap())
            .collect();
        Some(succs)
    }

    pub fn leaf_nodes(&self) -> impl Iterator<Item = &N> {
        self.graph
            .externals(Direction::Outgoing)
            .map(move |idx| self.graph.node_weight(idx).unwrap())
    }

    pub fn edge_weights(&self) -> impl Iterator<Item = &D> {
        self.ops.values()
    }

    /// Create a zero-copy view into this CFG starting from `origin`.
    /// The view contains all nodes reachable from `origin` (including `origin`).
    /// Returns `None` if `origin` is not in the CFG.
    pub fn view_from<T: Borrow<N>>(&self, origin: T) -> Option<PcodeCfgView<'_, N, D>>
    where
        N: std::hash::Hash + Eq,
    {
        let origin_key = origin.borrow();
        let &origin_idx = self.indices.get(origin_key)?;
        // Simple DFS/BFS to collect reachable nodes
        let mut stack = vec![origin_idx];
        let mut visited: std::collections::HashSet<NodeIndex> = std::collections::HashSet::new();
        while let Some(idx) = stack.pop() {
            if !visited.insert(idx) {
                continue;
            }
            for e in self.graph.edges_directed(idx, Direction::Outgoing) {
                stack.push(e.target());
            }
        }
        Some(PcodeCfgView {
            cfg: self,
            origin: origin_idx,
            nodes: visited,
        })
    }
}

impl PcodeStore for PcodeCfg<ConcretePcodeAddress, PcodeOperation> {
    fn get_pcode_op_at<T: Borrow<ConcretePcodeAddress>>(&self, addr: T) -> Option<PcodeOperation> {
        let addr = *addr.borrow();
        self.get_op_at(addr).cloned()
    }
}

impl<N: CfgState> PcodeCfg<N, PcodeOperation> {
    pub fn basic_blocks(&self) -> PcodeCfg<N, Vec<PcodeOperation>> {
        use petgraph::visit::EdgeRef;
        // Step 1: Initialize new graph and maps
        let mut graph = DiGraph::<N, EmptyEdge>::default();
        let mut ops: HashMap<N, Vec<PcodeOperation>> = HashMap::new();
        let mut indices: HashMap<N, NodeIndex> = HashMap::new();
        // Step 2: Wrap each op in a Vec and add nodes
        for node in self.graph.node_indices() {
            let n = self.graph.node_weight(node).unwrap().clone();
            let op = self
                .ops
                .get(&n)
                .map(|op| vec![op.clone()])
                .unwrap_or_default();
            let idx = graph.add_node(n.clone());
            graph.add_node(n.clone());
            indices.insert(n.clone(), idx);
            ops.insert(n, op);
        }
        // Step 3: Add edges
        for edge in self.graph.edge_indices() {
            let (fromidx, toidx) = self.graph.edge_endpoints(edge).unwrap();
            let from = self.graph.node_weight(fromidx).unwrap();
            let to = self.graph.node_weight(toidx).unwrap();
            let from_idx = *indices.get(from).unwrap();
            let to_idx = *indices.get(to).unwrap();
            graph.add_edge(from_idx, to_idx, EmptyEdge);
        }
        // Step 4: Wait-list algorithm for merging
        let mut changed = true;
        while changed {
            changed = false;
            for node in graph.node_indices() {
                // Only consider nodes still present
                let out_edges: Vec<_> = graph.edges_directed(node, Direction::Outgoing).collect();
                if out_edges.len() != 1 {
                    continue;
                }
                let edge = out_edges[0];
                let target = edge.target();
                let in_edges: Vec<_> = graph.edges_directed(target, Direction::Incoming).collect();
                if in_edges.len() != 1 {
                    continue;
                }

                // Merge target into node
                let src_n = graph.node_weight(node).unwrap().clone();
                let tgt_n = graph.node_weight(target).unwrap().clone();
                // Fix borrow: collect target ops first
                let tgt_ops = ops.get(&tgt_n).cloned().unwrap_or_default();
                if !tgt_ops.is_empty() {
                    ops.entry(src_n.clone()).or_default().extend(tgt_ops);
                }
                // Redirect outgoing edges of target to source
                let tgt_out_edges: Vec<_> = graph
                    .edges_directed(target, Direction::Outgoing)
                    .map(|e| e.target())
                    .collect();
                for tgt_out in tgt_out_edges {
                    graph.add_edge(node, tgt_out, EmptyEdge);
                }
                // Remove target node and its ops
                graph.remove_node(target);
                ops.remove(&tgt_n);
                indices.remove(&tgt_n);
                changed = true;
                break; // Restart after each merge
            }
        }
        // Step 5: Build a new graph using only connected nodes
        let mut new_graph = DiGraph::<N, EmptyEdge>::default();
        let mut new_ops: HashMap<N, Vec<PcodeOperation>> = HashMap::new();
        let mut new_indices: HashMap<N, NodeIndex> = HashMap::new();
        // Collect connected nodes
        let connected_nodes: Vec<_> = graph
            .node_indices()
            .filter(|&node| {
                graph
                    .edges_directed(node, Direction::Incoming)
                    .next()
                    .is_some()
                    || graph
                        .edges_directed(node, Direction::Outgoing)
                        .next()
                        .is_some()
            })
            .collect();
        // Add connected nodes to new graph
        for node in connected_nodes.iter() {
            let n = graph.node_weight(*node).unwrap().clone();
            let idx = new_graph.add_node(n.clone());
            new_indices.insert(n.clone(), idx);
            if let Some(op) = ops.get(&n) {
                new_ops.insert(n, op.clone());
            }
        }
        // Add edges between connected nodes
        for edge in graph.edge_indices() {
            let (fromidx, toidx) = graph.edge_endpoints(edge).unwrap();
            if connected_nodes.contains(&fromidx) && connected_nodes.contains(&toidx) {
                let from = graph.node_weight(fromidx).unwrap();
                let to = graph.node_weight(toidx).unwrap();
                let from_idx = *new_indices.get(from).unwrap();
                let to_idx = *new_indices.get(to).unwrap();
                new_graph.add_edge(from_idx, to_idx, EmptyEdge);
            }
        }
        // Step 6: Build and return new PcodeCfg
        PcodeCfg {
            graph: new_graph,
            ops: new_ops,
            indices: new_indices,
            models: self.models.clone() // todo: just include the ones that remain
        }
    }
}

type UnwoundPCodeCfgView<'a, D> = PcodeCfgView<'a, UnwoundLocation, D>;

impl<'a, D: ModelTransition<UnwoundLocationModel>> UnwoundPCodeCfgView<'a, D>{
    pub fn check_model(&self, location: UnwoundLocation, ctl_model: CtlFormula) -> Result<Bool, JingleError>{
        self.
    }
}