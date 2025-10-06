use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{OpCode, PcodeOperation, SleighArchInfo};
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
use jingle_sleigh::PcodeOperation::Branch;

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
    graph: DiGraph<N, EmptyEdge>,
    ops: HashMap<N, D>,
    indices: HashMap<N, NodeIndex>,
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

    pub fn nodes(&self) -> impl Iterator<Item = &N> {
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
        self.graph.add_edge(from_idx, to_idx, EmptyEdge);
    }

    pub fn leaf_nodes(&self) -> impl Iterator<Item = &N> {
        self.graph
            .externals(Direction::Outgoing)
            .map(move |idx| self.graph.node_weight(idx).unwrap())
    }

    pub fn edge_weights(&self) -> impl Iterator<Item = &D> {
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
            loc_eq.implies(from_state_final.mem_eq(to_state).simplify())
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
        use std::collections::{HashMap, HashSet};

        // Step 1: Identify block boundaries
        let mut block_starts = HashSet::new();
        let mut block_ends = HashSet::new();
        let mut preds: HashMap<&N, usize> = HashMap::new();
        let mut succs: HashMap<&N, usize> = HashMap::new();
        for node in self.graph.node_indices() {
            let n = self.graph.node_weight(node).unwrap();
            let pred_count = self.graph.edges_directed(node, Direction::Incoming).count();
            let succ_count = self.graph.edges_directed(node, Direction::Outgoing).count();
            preds.insert(n, pred_count);
            succs.insert(n, succ_count);
        }
        // Entry node is always a block start
        if let Some(entry) = self.graph.node_indices().next() {
            let entry_n = self.graph.node_weight(entry).unwrap();
            block_starts.insert(entry_n.clone());
        }
        // Nodes with multiple preds (join) or succs (branch) are block boundaries
        for (n, &pred_count) in &preds {
            if pred_count > 1 {
                block_starts.insert((*n).clone());
            }
        }
        for (n, &succ_count) in &succs {
            if succ_count > 1 {
                block_ends.insert((*n).clone());
            }
        }
        // Step 2: Traverse and group nodes into blocks
        let mut blocks: Vec<(Vec<N>, Vec<PcodeOperation>)> = Vec::new();
        let mut visited = HashSet::new();
        for node in self.graph.node_indices() {
            let mut block_nodes = Vec::new();
            let mut block_ops = Vec::new();
            let mut current = node;
            while !visited.contains(&current) {
                visited.insert(current);
                let n = self.graph.node_weight(current).unwrap().clone();
                block_nodes.push(n.clone());
                if let Some(op) = self.ops.get(&n) {
                    // Filter out branch instructions
                    if op.opcode() != OpCode::CPUI_BRANCH {
                        block_ops.push(op.clone());
                    }
                }
                // End block if this is a block end or has multiple outgoing edges
                let succ_count = self
                    .graph
                    .edges_directed(current, Direction::Outgoing)
                    .count();
                if block_ends.contains(&n) || succ_count != 1 {
                    break;
                }
                // Move to next node
                let mut next_nodes = self
                    .graph
                    .edges_directed(current, Direction::Outgoing)
                    .map(|e| e.target());
                if let Some(next) = next_nodes.next() {
                    current = next;
                } else {
                    break;
                }
            }
            if !block_nodes.is_empty() {
                blocks.push((block_nodes, block_ops));
            }
        }
        // Step 3: Build new graph
        let mut bb_cfg = PcodeCfg::<N, Vec<PcodeOperation>>::default();
        let mut block_map: HashMap<N, usize> = HashMap::new();
        for (i, (nodes, ops)) in blocks.iter().enumerate() {
            // Use first node as block id
            let block_id = nodes[0].clone();
            bb_cfg.add_node(block_id.clone());
            bb_cfg.ops.insert(block_id.clone(), ops.clone());
            for n in nodes {
                block_map.insert(n.clone(), i);
            }
        }
        // Add edges between blocks
        for (i, (nodes, _)) in blocks.iter().enumerate() {
            let last_node = nodes.last().unwrap();
            let last_idx = *self.indices.get(last_node).unwrap();
            for edge in self.graph.edges_directed(last_idx, Direction::Outgoing) {
                let target = self.graph.node_weight(edge.target()).unwrap();
                if let Some(&target_block) = block_map.get(target) {
                    if target_block != i {
                        let from_block = nodes[0].clone();
                        let to_block = blocks[target_block].0[0].clone();
                        bb_cfg.graph.add_edge(
                            *bb_cfg.indices.get(&from_block).unwrap(),
                            *bb_cfg.indices.get(&to_block).unwrap(),
                            EmptyEdge,
                        );
                    }
                }
            }
        }
        bb_cfg
    }
}