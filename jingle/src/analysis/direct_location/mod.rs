use crate::analysis::Analysis;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::state::AbstractState;
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use petgraph::prelude::DiGraph;
use std::iter::{empty, once};

pub struct DirectLocationCPA<T> {
    pcode: T,
    pub graph: DiGraph<(ConcretePcodeAddress, PcodeOperation), ()>,
}

pub struct DirectLocationAnalysis;

impl<T: PcodeStore> DirectLocationCPA<T> {
    pub fn new(pcode: T) -> Self {
        Self {
            pcode,
            graph: Default::default(),
        }
    }

    pub fn pcode_at(
        &self,
        state: &<DirectLocationCPA<T> as ConfigurableProgramAnalysis>::State,
    ) -> Option<PcodeOperation> {
        state.value().and_then(|a| self.pcode.get_pcode_op_at(*a))
    }
}
impl<T: PcodeStore> ConfigurableProgramAnalysis for DirectLocationCPA<T> {
    type State = PcodeAddressLattice;

    type Iter = Box<dyn Iterator<Item = Self::State>>;

    fn successor_states(&mut self, state: &Self::State) -> Self::Iter {
        match state {
            PcodeAddressLattice::Value(a) => {
                if let Some(op) = self.pcode.get_pcode_op_at(*a) {
                    // let nd = self.graph.add_node((*a, op.clone()));
                    let iter: Vec<_> = state
                        .transfer(&op)
                        .inspect(|_f| {
                            // if let PcodeAddressLattice::Value(f) = f {
                            //     let nd2 = self.graph.add_node((*f, op));
                            //     self.graph.add_edge(nd, nd2, op.clone());
                            // }
                        })
                        .collect();
                    Box::new(iter.into_iter())
                } else {
                    Box::new(empty())
                }
            }
            PcodeAddressLattice::Top => Box::new(once(PcodeAddressLattice::Top)),
        }
    }
}

impl Analysis for DirectLocationAnalysis {
    type Output = PcodeCfg;
    type Input = ConcretePcodeAddress;

    fn run<T: PcodeStore>(&mut self, store: T, initial_state: Self::Input) -> Self::Output {
        let lattice = PcodeAddressLattice::Value(initial_state);
        let mut cpa = DirectLocationCPA::new(store);
        let _ = cpa.run_cpa(&lattice);
        PcodeCfg::new(cpa.graph.clone(), initial_state)
    }
    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input {
        addr
    }
}
