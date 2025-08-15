use crate::JingleContext;
use crate::modeling::machine::MachineState;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use petgraph::Direction;
use petgraph::graphmap::DiGraphMap;
use std::collections::HashMap;
use z3::ast::{Ast, Bool};
use z3::{Model, Solver};

pub struct PcodeCfg {
    graph: DiGraphMap<ConcretePcodeAddress, PcodeOperation>,
    #[expect(unused)]
    entry: ConcretePcodeAddress,
}

impl PcodeCfg {
    pub fn new(
        p0: DiGraphMap<ConcretePcodeAddress, PcodeOperation>,
        p1: ConcretePcodeAddress,
    ) -> PcodeCfg {
        Self {
            graph: p0,
            entry: p1,
        }
    }

    pub fn graph(&self) -> &DiGraphMap<ConcretePcodeAddress, PcodeOperation> {
        &self.graph
    }

    pub fn build_solver(&self, jingle: JingleContext) -> Solver {
        let solver = Solver::new(jingle.ctx());
        let mut states = HashMap::new();
        for addr in self.graph.nodes() {
            states.insert(addr, MachineState::fresh_for_address(&jingle, addr));
        }

        for addr in self.graph.nodes() {
            let outgoing: Vec<_> = self
                .graph
                .edges_directed(addr, Direction::Incoming)
                .collect();
            let options: Vec<_> = outgoing
                .iter()
                .map(|(from, to, op)| {
                    let to_state = states.get(to).expect("From state not found");
                    let from_state = states.get(from).expect("To state not found");
                    let relation = from_state.apply(op).unwrap();
                    let hi = relation.pc()._eq(to_state.pc());
                    hi.implies(&relation._eq(to_state))
                })
                .collect();
            if options.is_empty() {
                continue;
            }
            solver.assert(&Bool::or(jingle.ctx(), &options));
        }
        solver
    }
    pub fn build_model(&self, jingle: JingleContext) -> Model {
        let solver = self.build_solver(jingle);
        solver.check();
        solver.get_model().unwrap()
    }

    pub fn build_solver_implication(&self, jingle: JingleContext) -> Solver {
        let solver = Solver::new_for_logic(jingle.ctx(), "QF_ABV").unwrap();
        let mut states = HashMap::new();
        let mut post_states = HashMap::new();
        for addr in self.graph.nodes() {
            let s = MachineState::fresh_for_address(&jingle, dbg!(addr));
            states.insert(addr, s.clone());
            if let Some((_, _, op)) = self.graph.edges_directed(addr, Direction::Outgoing).next() {
                let f = s.apply(op).unwrap();
                post_states.insert(addr, f);
            }
        }

        let outgoing: Vec<_> = self.graph.all_edges().collect();
        let options: Vec<_> = outgoing
            .iter()
            .map(|(from, to, op)| {
                let from_state = states.get(from).expect("From state not found");
                let to_state = states.get(to).expect("To state not found");
                let from_state_final = from_state.apply(op).unwrap();
                let hi = from_state_final.pc()._eq(to_state.pc());
                hi.implies(&from_state_final._eq(to_state)).simplify()
            })
            .collect();

        solver.assert(&Bool::and(jingle.ctx(), &options));

        solver
    }
    pub fn build_model_implication(&self, jingle: JingleContext) -> Model {
        let solver = self.build_solver_implication(jingle);
        solver.check();
        solver.get_model().unwrap()
    }
}
