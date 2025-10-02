use crate::JingleError;
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::MachineState;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use petgraph::Direction;
use petgraph::graph::NodeIndex;
use petgraph::prelude::DiGraph;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use z3::ast::{Ast, Bool};
use z3::{Model, Params, Solver};

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

pub trait CfgStateModel {
    fn location_eq(&self, other: &Self) -> Bool;
    fn eq(&self, other: &Self) -> Bool;
}

impl CfgStateModel for MachineState {
    fn location_eq(&self, other: &Self) -> Bool {
        self.pc().eq(other.pc())
    }

    fn eq(&self, other: &Self) -> Bool {
        self.eq(other)
    }
}

pub trait CfgState: Clone + Hash + Eq {
    type Model: CfgStateModel + Clone;

    fn fresh(&self, i: &SleighArchInfo) -> Self::Model;
}

pub trait ModelTransition<S: CfgState>: Clone {
    fn transition(&self, init: &S::Model) -> Result<S::Model, JingleError>;
}

impl CfgState for ConcretePcodeAddress {
    type Model = MachineState;

    fn fresh(&self, i: &SleighArchInfo) -> Self::Model {
        MachineState::fresh_for_address(i, *self)
    }
}

impl ModelTransition<ConcretePcodeAddress> for PcodeOperation {
    fn transition(&self, init: &MachineState) -> Result<MachineState, JingleError> {
        init.apply(self)
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
}

impl<N: CfgState, D: ModelTransition<N>> PcodeCfg<N, D> {
    pub fn test_build<T: Borrow<SleighArchInfo>>(&self, info: T) -> Solver {
        let info = info.borrow();
        let solver = Solver::new_for_logic("QF_ABV").unwrap();
        let mut params = Params::new();
        params.set_bool("smt.array.extensional", false);
        solver.set_params(&params);
        let mut states = HashMap::new();
        let mut post_states = HashMap::new();
        for (addr, idx) in &self.indices {
            let op = &self.ops[addr];
            let s = addr.fresh(info);
            states.insert(addr, s.clone());
            if self
                .graph
                .edges_directed(*idx, Direction::Outgoing)
                .next()
                .is_some()
            {
                let f = op.transition(&s).unwrap();
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

                let from_state_final = post_states.get(from).unwrap();
                let to_state = states.get(to).expect("To state not found");
                let loc_eq = from_state_final.location_eq(to_state);
                loc_eq.implies(from_state_final.eq(to_state))
            })
            .collect();

        solver.assert(Bool::and(&options));

        solver
    }
}

impl PcodeCfg<ConcretePcodeAddress, PcodeOperation> {
    pub fn build_solver_implication<T: Borrow<SleighArchInfo>>(&self, info: T) -> Solver {
        let info = info.borrow();
        let solver = Solver::new_for_logic("QF_ABV").unwrap();
        let mut states = HashMap::new();
        let mut post_states = HashMap::new();
        for (addr, idx) in &self.indices {
            let op = &self.ops[addr];
            let s = MachineState::fresh_for_address(info, addr);
            states.insert(addr, s.clone());
            if self
                .graph
                .edges_directed(*idx, Direction::Outgoing)
                .next()
                .is_some()
            {
                let f = s.apply(op).unwrap();
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
    pub fn build_model_implication<T: Borrow<SleighArchInfo>>(&self, info: T) -> Model {
        let solver = self.build_solver_implication(info);
        solver.check();
        solver.get_model().unwrap()
    }
}

impl PcodeStore for PcodeCfg<ConcretePcodeAddress, PcodeOperation> {
    fn get_pcode_op_at<T: Borrow<ConcretePcodeAddress>>(&self, addr: T) -> Option<PcodeOperation> {
        let addr = *addr.borrow();
        self.get_op_at(addr).cloned()
    }
}
