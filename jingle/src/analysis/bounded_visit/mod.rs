mod state;

use crate::analysis::Analysis;
use crate::analysis::bounded_visit::state::BoundedStepsState;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::lattice::flat::FlatLattice::Value;
use crate::analysis::cpa::{ConfigurableProgramAnalysis, RunnableConfigurableProgramAnalysis};
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;

struct BoundedStepsCpa {
    cfg: PcodeCfg<ConcretePcodeAddress, PcodeOperation>,
}

impl BoundedStepsCpa {
    pub fn new<T: PcodeStore>(pcode: &T) -> Self {
        let info = pcode.info();
        Self {
            cfg: PcodeCfg::new(info),
        }
    }
}

impl ConfigurableProgramAnalysis for BoundedStepsCpa {
    type State = BoundedStepsState;

    fn reduce(&mut self, state: &Self::State, dest_state: &Self::State, op: &Option<PcodeOperation>) {
        if let Value(state) = state.location {
            self.cfg.add_node(state);
            if let Some(op) = op {
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
        let mut cpa = BoundedStepsCpa::new(&store);
        let _ = cpa.run_cpa(initial_state.into(), &store);
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
