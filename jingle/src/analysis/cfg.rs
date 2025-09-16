use crate::JingleContext;
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::MachineState;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use petgraph::Direction;
use petgraph::graph::NodeIndex;
use petgraph::prelude::{DiGraph, EdgeRef};
use std::borrow::Borrow;
use std::collections::HashMap;
use z3::ast::{Ast, Bool};
use z3::{Model, Solver};

#[derive(Debug, Default)]
pub struct PcodeCfg {
    graph: DiGraph<ConcretePcodeAddress, ()>,
    ops: HashMap<ConcretePcodeAddress, PcodeOperation>,
    indices: HashMap<ConcretePcodeAddress, NodeIndex>,
}

impl PcodeCfg {
    pub fn new() -> PcodeCfg {
        Self {
            graph: Default::default(),
            ops: Default::default(),
            indices: Default::default(),
        }
    }

    pub fn graph(&self) -> &DiGraph<ConcretePcodeAddress, ()> {
        &self.graph
    }

    pub fn addresses(&self) -> impl Iterator<Item = &ConcretePcodeAddress> {
        self.indices.keys()
    }

    pub fn get_op_at<T: Borrow<ConcretePcodeAddress>>(&self, addr: T) -> Option<&PcodeOperation> {
        self.ops.get(addr.borrow())
    }

    pub fn leaf_nodes(&self) -> impl Iterator<Item = &ConcretePcodeAddress> {
        self.graph
            .node_indices()
            .filter(move |node| {
                self.graph
                    .neighbors_directed(*node, Direction::Outgoing)
                    .next()
                    .is_none()
            })
            .map(|node| self.graph.node_weight(node).unwrap())
    }

    pub fn add_node<T: Borrow<ConcretePcodeAddress>>(&mut self, node: T) {
        let node = node.borrow().clone();
        if !self.indices.contains_key(&node) {
            let idx = self.graph.add_node(node);
            self.indices.insert(node, idx);
        }
    }

    pub fn add_edge<A, B, C>(&mut self, from: A, to: B, op: C)
    where
        A: Borrow<ConcretePcodeAddress>,
        B: Borrow<ConcretePcodeAddress>,
        C: Borrow<PcodeOperation>,
    {
        let from = from.borrow();
        let to = to.borrow();
        let op = op.borrow();
        self.add_node(from);
        self.add_node(to);
        self.ops.insert(from.clone(), op.clone());
        let from_idx = *self.indices.get(&from).unwrap();
        let to_idx = *self.indices.get(&to).unwrap();
        self.graph.add_edge(from_idx, to_idx, ());
    }

    pub fn build_solver(&self, jingle: JingleContext) -> Solver {
        let solver = Solver::new();
        let mut states = HashMap::new();
        for (addr, _) in &self.indices {
            states.insert(addr, MachineState::fresh_for_address(&jingle, addr.clone()));
        }

        for idx in self.graph.node_indices() {
            let outgoing: Vec<_> = self
                .graph
                .edges_directed(idx, Direction::Incoming)
                .map(|e| e.id())
                .collect();
            let options: Vec<_> = outgoing
                .iter()
                .map(|edge| {
                    let (fromidx, toidx) = self.graph.edge_endpoints(*edge).unwrap();
                    let from = self.graph.node_weight(fromidx).unwrap();
                    let op = self.ops.get(from).unwrap();
                    let to = self.graph.node_weight(toidx).unwrap();
                    let to_state = states.get(to).expect("From state not found");
                    let from_state = states.get(from).expect("To state not found");
                    let relation = from_state.apply(op).unwrap();
                    let hi = relation.pc().eq(to_state.pc());
                    hi.implies(relation.eq(to_state))
                })
                .collect();
            if options.is_empty() {
                continue;
            }
            solver.assert(Bool::or(&options));
        }
        solver
    }
    pub fn build_model(&self, jingle: JingleContext) -> Model {
        let solver = self.build_solver(jingle);
        solver.check();
        solver.get_model().unwrap()
    }

    pub fn build_solver_implication(&self, jingle: JingleContext) -> Solver {
        let solver = Solver::new_for_logic("QF_ABV").unwrap();
        let mut states = HashMap::new();
        let mut post_states = HashMap::new();
        for (addr, idx) in &self.indices {
            let op = &self.ops[&addr];
            let s = MachineState::fresh_for_address(&jingle, addr);
            states.insert(addr, s.clone());
            if self
                .graph
                .edges_directed(*idx, Direction::Outgoing)
                .next()
                .is_some()
            {
                let f = s.apply(&op).unwrap();
                post_states.insert(addr, f);
            }
        }

        let options: Vec<_> = self
            .graph
            .edge_indices()
            .map(|edge| {
                let (fromidx, toidx) = self.graph.edge_endpoints(edge).unwrap();
                let from = self.graph.node_weight(fromidx).unwrap();
                let to = self.graph.node_weight(toidx).unwrap();
                let op = self.ops.get(from).unwrap();

                let from_state = states.get(from).expect("From state not found");
                let to_state = states.get(to).expect("To state not found");
                let from_state_final = from_state.apply(op).unwrap();
                let hi = from_state_final.pc().eq(to_state.pc());
                hi.implies(from_state_final.eq(to_state)).simplify()
            })
            .collect();

        solver.assert(Bool::and(&options));

        solver
    }
    pub fn build_model_implication(&self, jingle: JingleContext) -> Model {
        let solver = self.build_solver_implication(jingle);
        solver.check();
        solver.get_model().unwrap()
    }
}

impl PcodeStore for PcodeCfg {
    fn get_pcode_op_at<T: Borrow<ConcretePcodeAddress>>(&self, addr: T) -> Option<PcodeOperation> {
        let addr = *addr.borrow();
        self.get_op_at(addr).cloned()
    }
}
