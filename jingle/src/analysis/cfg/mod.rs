use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use petgraph::Direction;
use petgraph::graph::NodeIndex;
use petgraph::prelude::DiGraph;
use petgraph::visit::EdgeRef;
pub use state::{CfgState, CfgStateModel, ModelTransition};
use std::borrow::Borrow;
use std::collections::HashMap;
use z3::ast::{Ast, Bool};
use z3::{Params, Solver};

mod state;

#[derive(Debug)]
pub struct PcodeCfg<N, D> {
    graph: DiGraph<N, ()>,
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

    pub fn graph(&self) -> &DiGraph<N, ()> {
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
        self.graph.add_edge(from_idx, to_idx, ());
    }

    pub fn leaf_nodes(&self) -> impl Iterator<Item = &N> {
        self.graph
            .externals(Direction::Outgoing)
            .map(move |idx| self.graph.node_weight(idx).unwrap())
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
