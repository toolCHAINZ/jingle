use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use petgraph::Direction;
use petgraph::graph::NodeIndex;
use petgraph::prelude::DiGraph;
use petgraph::visit::{EdgeRef, NodeRef};
pub use state::{CfgState, CfgStateModel, ModelTransition};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{Formatter, LowerHex};
use z3::ast::{Ast, Bool};
use z3::{Params, Solver};

mod state;

#[derive(Debug, Default, Copy, Clone, Hash)]
pub struct EmptyEdge;

impl LowerHex for EmptyEdge {
    fn fmt(&self, _: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

#[derive(Debug)]
pub struct PcodeCfg<N, D> {
    pub(crate) graph: DiGraph<N, EmptyEdge>,
    pub(crate) ops: HashMap<N, D>,
    pub(crate) indices: HashMap<N, NodeIndex>,
}

impl<N, D> Default for PcodeCfg<N, D> {
    fn default() -> Self {
        Self {
            graph: Default::default(),
            ops: Default::default(),
            indices: Default::default(),
        }
    }
}

impl<N: CfgState, D: ModelTransition<N>> PcodeCfg<N, D> {
    pub fn new() -> Self {
        Self {
            graph: Default::default(),
            ops: Default::default(),
            indices: Default::default(),
        }
    }

    pub fn graph(&self) -> &DiGraph<N, EmptyEdge> {
        &self.graph
    }

    pub fn nodes(&self) -> impl Iterator<Item=&N> {
        self.indices.keys()
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

    pub fn leaf_nodes(&self) -> impl Iterator<Item=&N> {
        self.graph
            .externals(Direction::Outgoing)
            .map(move |idx| self.graph.node_weight(idx).unwrap())
    }

    pub fn edge_weights(&self) -> impl Iterator<Item=&D> {
        self.ops.values()
    }


}

impl<N: CfgState, D: ModelTransition<N>> PcodeCfg<N, D> {
    pub fn test_build<T: Borrow<SleighArchInfo>>(&self, info: T) -> Solver {
        let info = info.borrow();
        let solver = Solver::new();
        let mut params = Params::new();
        params.set_bool("smt.array.extensional", false);
        solver.set_params(&params);
        let mut states = HashMap::new();
        let mut post_states = HashMap::new();
        for (addr, idx) in &self.indices {
            let s = addr.fresh(info);
            states.insert(addr, s.clone());
            if self
                .graph
                .edges_directed(*idx, Direction::Outgoing)
                .next()
                .is_some()
            {
                let op = &self.ops[addr];
                let f = op.transition(&s).unwrap();
                post_states.insert(addr, f);
            }
        }

        let options = self.graph.edge_indices().map(|edge| {
            let (fromidx, toidx) = self.graph.edge_endpoints(edge).unwrap();
            let from = self.graph.node_weight(fromidx).unwrap();
            let to = self.graph.node_weight(toidx).unwrap();

            let from_state_final = post_states.get(from).unwrap();
            let to_state = states.get(to).expect("To state not found");
            let loc_eq = from_state_final.location_eq(to_state).simplify();
            loc_eq.implies(from_state_final.mem_eq(to_state))
        });
        for x in options {
            solver.assert(x);
        }
        for node in self.graph.node_indices() {
            let edges = self.graph.edges_directed(node, Direction::Outgoing);
            let b = edges.map(|e| {
                let from_weight = self.graph.node_weight(e.source()).unwrap();
                let to_weight = self.graph.node_weight(e.target()).unwrap();
                let from_state_final = post_states.get(from_weight).unwrap();
                let to_state = states.get(to_weight).unwrap();
                let loc_eq = from_state_final.location_eq(to_state);
                loc_eq
            });
            let b = &b.collect::<Vec<_>>();
            if b.len() > 0 {
                let bool = if b.len() > 1 {
                    Bool::or(&b)
                } else {
                    b[0].clone()
                };
                solver.assert(&bool);
            }
        }
        solver
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
            let op = self.ops.get(&n).map(|op| vec![op.clone()]).unwrap_or_default();
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
                if out_edges.len() != 1 { continue; }
                let edge = out_edges[0];
                let target = edge.target();
                let in_edges: Vec<_> = graph.edges_directed(target, Direction::Incoming).collect();
                if in_edges.len() != 1 { continue; }

                // Merge target into node
                let src_n = graph.node_weight(node).unwrap().clone();
                let tgt_n = graph.node_weight(target).unwrap().clone();
                // Fix borrow: collect target ops first
                let tgt_ops = ops.get(&tgt_n).cloned().unwrap_or_default();
                if !tgt_ops.is_empty() {
                    ops.entry(src_n.clone()).or_default().extend(tgt_ops);
                }
                // Redirect outgoing edges of target to source
                let tgt_out_edges: Vec<_> = graph.edges_directed(target, Direction::Outgoing).map(|e| e.target()).collect();
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
        let connected_nodes: Vec<_> = graph.node_indices()
            .filter(|&node| {
                graph.edges_directed(node, Direction::Incoming).next().is_some() ||
                graph.edges_directed(node, Direction::Outgoing).next().is_some()
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