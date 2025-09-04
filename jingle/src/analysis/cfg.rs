use crate::JingleContext;
use crate::modeling::machine::MachineState;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use petgraph::Direction;
use petgraph::data::DataMap;
use petgraph::prelude::{DiGraph, EdgeRef};
use std::collections::HashMap;
use z3::ast::{Ast, Bool};
use z3::{Model, Solver};

pub struct PcodeCfg {
    graph: DiGraph<(ConcretePcodeAddress, PcodeOperation), ()>,
    #[expect(unused)]
    entry: ConcretePcodeAddress,
}

impl PcodeCfg {
    pub fn new(
        p0: DiGraph<(ConcretePcodeAddress, PcodeOperation), ()>,
        p1: ConcretePcodeAddress,
    ) -> PcodeCfg {
        Self {
            graph: p0,
            entry: p1,
        }
    }

    pub fn graph(&self) -> &DiGraph<(ConcretePcodeAddress, PcodeOperation), ()> {
        &self.graph
    }

    pub fn leaf_ndoes(&self) -> impl Iterator<Item = (ConcretePcodeAddress, PcodeOperation)> {
        self.graph
            .node_indices()
            .filter(|&n| {
                self.graph
                    .neighbors_directed(n, Direction::Outgoing)
                    .next()
                    .is_none()
            })
            .map(|n| self.graph.node_weight(n).unwrap())
            .cloned()
    }

    pub fn build_solver(&self, jingle: JingleContext) -> Solver {
        let solver = Solver::new();
        let mut states = HashMap::new();
        for addr in self.graph.node_indices() {
            let (addr, _) = self.graph.node_weight(addr).unwrap();
            states.insert(addr, MachineState::fresh_for_address(&jingle, *addr));
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
                    let (from, op) = self.graph.node_weight(fromidx).unwrap();
                    let (to, _) = self.graph.node_weight(toidx).unwrap();
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
        for idx in self.graph.node_indices() {
            let (addr, op) = self.graph.node_weight(idx).unwrap().clone();
            let s = MachineState::fresh_for_address(&jingle, addr);
            states.insert(addr, s.clone());
            if let Some(_) = self.graph.edges_directed(idx, Direction::Outgoing).next() {
                let f = s.apply(&op).unwrap();
                post_states.insert(addr, f);
            }
        }

        let options: Vec<_> = self
            .graph
            .edge_indices()
            .map(|edge| {
                let (fromidx, toidx) = self.graph.edge_endpoints(edge).unwrap();
                let (from, op) = self.graph.node_weight(fromidx).unwrap();
                let (to, _) = self.graph.node_weight(toidx).unwrap();

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
