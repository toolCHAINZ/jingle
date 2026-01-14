use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
pub use model::{CfgState, CfgStateModel, ModelTransition};
use petgraph::Direction;
use petgraph::graph::NodeIndex;
use petgraph::prelude::StableDiGraph;
use petgraph::visit::EdgeRef;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{Formatter, LowerHex};
use std::rc::Rc;

pub(crate) mod model;

#[derive(Debug, Default, Copy, Clone, Hash)]
pub struct EmptyEdge;

impl LowerHex for EmptyEdge {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

#[derive(Debug)]
pub struct PcodeCfg<N: CfgState = ConcretePcodeAddress, D = PcodeOperation> {
    pub(crate) graph: StableDiGraph<N, EmptyEdge>,
    pub(crate) ops: HashMap<N, D>,
    pub(crate) indices: HashMap<N, NodeIndex>,
}

#[derive(Debug)]
pub struct ModeledPcodeCfg<N: CfgState = ConcretePcodeAddress, D = PcodeOperation> {
    pub(crate) cfg: PcodeCfg<N, D>,
    #[allow(unused)]
    pub(crate) info: SleighArchInfo,
    pub(crate) models: HashMap<N, N::Model>,
}

#[derive(Clone)]
pub struct PcodeCfgVisitor<'a, N: CfgState = ConcretePcodeAddress, D = PcodeOperation> {
    cfg: &'a ModeledPcodeCfg<N, D>,
    location: N,
    pub(crate) visited_locations: Rc<RefCell<HashSet<N>>>,
}

impl<'a, N: CfgState, D: ModelTransition<N::Model>> PcodeCfgVisitor<'a, N, D> {
    pub(crate) fn successors(&mut self) -> impl Iterator<Item = Self> {
        self.cfg
            .cfg
            .successors(&self.location)
            .into_iter()
            .flatten()
            .flat_map(|n| {
                // Use a short-lived borrow so the RefMut is released before we construct the new visitor
                let is_repeat = {
                    let mut set = self.visited_locations.borrow_mut();
                    if set.contains(n) {
                        true
                    } else {
                        set.insert(n.clone());
                        false
                    }
                };

                if is_repeat {
                    None
                } else {
                    Some(Self {
                        cfg: self.cfg,
                        location: n.clone(),
                        visited_locations: self.visited_locations.clone(),
                    })
                }
            })
    }

    pub(crate) fn transition(&self) -> Option<&D> {
        self.cfg.cfg.ops.get(&self.location)
    }

    pub fn location(&self) -> &N {
        &self.location
    }

    pub fn state(&self) -> Option<&N::Model> {
        self.cfg.models.get(&self.location)
    }
}

impl<N: CfgState, D: ModelTransition<N::Model>> Default for PcodeCfg<N, D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<N: CfgState, D: ModelTransition<N::Model>> PcodeCfg<N, D> {
    pub fn new() -> Self {
        Self {
            graph: Default::default(),
            ops: Default::default(),
            indices: Default::default(),
        }
    }

    pub fn graph(&self) -> &StableDiGraph<N, EmptyEdge> {
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

    pub fn replace_and_combine_nodes<T: Borrow<N>, S: Borrow<N>>(
        &mut self,
        old_weight: T,
        new_weight: S,
    ) {
        // Copy the indices first to avoid borrow issues
        let old_idx = self.indices.get(old_weight.borrow()).copied();
        let new_idx = self.indices.get(new_weight.borrow()).copied();

        tracing::debug!("replace_and_combine_nodes called: old_idx={:?}, new_idx={:?}", old_idx, new_idx);

        if let (Some(old_idx), Some(new_idx)) = (old_idx, new_idx) {
            // If the indices are the same, the nodes are already merged - nothing to do
            if old_idx == new_idx {
                tracing::debug!("Indices are identical, skipping merge");
                return;
            }

            tracing::debug!("Both nodes found with different indices, proceeding with merge");
            // We are going to keep the old index, but replace its weight with new_weight
            // All edges from new_idx will be redirected to old_idx, then new_idx is removed

            // Redirect all incoming edges from new_idx to old_idx
            let incoming: Vec<_> = self
                .graph
                .edges_directed(new_idx, Direction::Incoming)
                .map(|edge| edge.source())
                .collect();
            for source in incoming {
                if !self.graph.contains_edge(source, old_idx) {
                    self.graph.add_edge(source, old_idx, EmptyEdge);
                }
            }

            // Redirect all outgoing edges from new_idx to old_idx
            let outgoing: Vec<_> = self
                .graph
                .edges_directed(new_idx, Direction::Outgoing)
                .map(|edge| edge.target())
                .collect();
            for target in &outgoing {
                if !self.graph.contains_edge(old_idx, *target) {
                    self.graph.add_edge(old_idx, *target, EmptyEdge);
                }
            }

            // Remove the new node from the graph (using StableGraph so indices remain valid)
            self.graph.remove_node(new_idx);

            // Update the weight at old_idx to be new_weight
            if let Some(node_weight) = self.graph.node_weight_mut(old_idx) {
                *node_weight = new_weight.borrow().clone();
            }

            // Update the indices map: new_weight should now map to old_idx
            self.indices.insert(new_weight.borrow().clone(), old_idx);
            self.indices.remove(old_weight.borrow());

            // Update the ops map: prefer the op from new_weight if it exists, otherwise use old_weight's op
            let op_to_keep = self.ops.get(new_weight.borrow())
                .or_else(|| self.ops.get(old_weight.borrow()))
                .cloned();

            self.ops.remove(old_weight.borrow());
            self.ops.remove(new_weight.borrow());

            if let Some(op) = op_to_keep {
                self.ops.insert(new_weight.borrow().clone(), op);
            }else{
                dbg!("Missing op!");
            }
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

    /// Return predecessors of a node (by value) as references into the backing CFG.
    /// Returns `None` if the node is not present in the CFG.
    pub fn predecessors<T: Borrow<N>>(&self, node: T) -> Option<Vec<&N>> {
        let n = node.borrow();
        let idx = *self.indices.get(n)?;
        let preds: Vec<&N> = self
            .graph
            .edges_directed(idx, Direction::Incoming)
            .map(|e| self.graph.node_weight(e.source()).unwrap())
            .collect();
        Some(preds)
    }

    pub fn leaf_nodes(&self) -> impl Iterator<Item = &N> {
        self.graph
            .externals(Direction::Outgoing)
            .map(move |idx| self.graph.node_weight(idx).unwrap())
    }

    pub fn edge_weights(&self) -> impl Iterator<Item = &D> {
        self.ops.values()
    }

    pub fn nodes_for_location<S: PartialEq<N>>(&self, location: S) -> impl Iterator<Item = &N> {
        self.nodes().filter(move |a| location == **a)
    }

    /// Create a `ModeledPcodeCfg` by generating SMT models for all nodes in the CFG.
    pub fn smt_model(self, info: SleighArchInfo) -> ModeledPcodeCfg<N, D> {
        ModeledPcodeCfg::new(self, info)
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
        let mut graph = StableDiGraph::<N, EmptyEdge>::default();
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
        let mut new_graph = StableDiGraph::<N, EmptyEdge>::default();
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
        }
    }
}

impl<N: CfgState, D: ModelTransition<N::Model>> ModeledPcodeCfg<N, D> {
    pub fn new(cfg: PcodeCfg<N, D>, info: SleighArchInfo) -> Self {
        let mut models = HashMap::new();
        for node in cfg.nodes() {
            let model = node.new_const(&info);
            models.insert(node.clone(), model);
        }
        Self { cfg, models, info }
    }

    pub fn cfg(&self) -> &PcodeCfg<N, D> {
        &self.cfg
    }

    pub fn models(&self) -> &HashMap<N, N::Model> {
        &self.models
    }

    // Delegation methods to underlying PcodeCfg
    pub fn graph(&self) -> &StableDiGraph<N, EmptyEdge> {
        self.cfg.graph()
    }

    pub fn nodes(&self) -> impl Iterator<Item = &N> {
        self.cfg.nodes()
    }

    pub fn leaf_nodes(&self) -> impl Iterator<Item = &N> {
        self.cfg.leaf_nodes()
    }

    pub fn edge_weights(&self) -> impl Iterator<Item = &D> {
        self.cfg.edge_weights()
    }

    pub fn nodes_for_location<S: PartialEq<N>>(&self, location: S) -> impl Iterator<Item = &N> {
        self.cfg.nodes_for_location(location)
    }
}
