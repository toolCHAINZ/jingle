mod state;

use crate::analysis::Analysis;
use crate::analysis::bounded_visit::state::BoundedStepsState;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::flat::FlatLattice::Value;
use crate::analysis::cpa::state::{AbstractState, Successor};
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;

struct BoundedStepsCpa<T: PcodeStore> {
    pcode: T,
    cfg: PcodeCfg<ConcretePcodeAddress, PcodeOperation>,
}

impl<T: PcodeStore> BoundedStepsCpa<T> {
    pub fn new(pcode: T) -> Self {
        Self {
            pcode,
            cfg: Default::default(),
        }
    }
}

impl<T: PcodeStore> ConfigurableProgramAnalysis for BoundedStepsCpa<T> {
    type State = BoundedStepsState;

    fn successor_states<'a>(&self, state: &'a Self::State) -> Successor<'a, Self::State> {
        let opt = state.location.value().cloned();
        if let Some(addr) = opt {
            let iter = self.pcode.get_pcode_op_at(addr);
            iter.into_iter()
                .flat_map(|op| state.transfer(&op).into_iter())
                .into()
        } else {
            std::iter::empty().into()
        }
    }

    fn reduce(&mut self, state: &Self::State, dest_state: &Self::State) {
        if let Value(state) = state.location {
            self.cfg.add_node(state);
            if let Some(op) = self.pcode.get_pcode_op_at(state) {
                if let Value(dest_state) = dest_state.location {
                    self.cfg.add_edge(state, dest_state, op.clone());
                }
            }
        }
    }
}

pub struct BoundedStepLocationAnalysis {
    max_steps: usize,
}

impl Analysis for BoundedStepLocationAnalysis {
    type Output = PcodeCfg<ConcretePcodeAddress, PcodeOperation>;
    type Input = BoundedStepsState;

    fn run<T: PcodeStore, I: Into<Self::Input>>(
        &mut self,
        store: T,
        initial_state: I,
    ) -> Self::Output {
        let mut cpa = BoundedStepsCpa::new(store);
        let _ = cpa.run_cpa(initial_state.into());
        cpa.cfg
    }

    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input {
        BoundedStepsState::new(addr.into(), self.max_steps)
    }
}

impl BoundedStepLocationAnalysis {
    pub fn new(max_steps: usize) -> Self {
        Self { max_steps }
    }
}
