mod state;

use crate::analysis::Analysis;
use crate::analysis::bounded_visit::state::BoundedStepsState;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::flat::FlatLattice::Value;
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;

struct BoundedStepsCpa<T: PcodeStore> {
    pcode: T,
    cfg: PcodeCfg<ConcretePcodeAddress, PcodeOperation>,
}

impl<T: PcodeStore> BoundedStepsCpa<T> {
    pub fn new(pcode: T) -> Self {
        let info = pcode.info();
        Self {
            pcode,
            cfg: PcodeCfg::new(info),
        }
    }
}

impl<T: PcodeStore> ConfigurableProgramAnalysis for BoundedStepsCpa<T> {
    type State = BoundedStepsState;

    fn get_pcode_store(&self) -> &impl PcodeStore {
        &self.pcode
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
