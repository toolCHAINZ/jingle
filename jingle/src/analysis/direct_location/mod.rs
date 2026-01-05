use crate::analysis::Analysis;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::{ConfigurableProgramAnalysis, RunnableConfigurableProgramAnalysis};
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;

pub struct DirectLocationCPA {
    cfg: PcodeCfg<ConcretePcodeAddress, PcodeOperation>,
}

impl DirectLocationCPA {
    pub fn cfg(&self) -> &PcodeCfg<ConcretePcodeAddress, PcodeOperation> {
        &self.cfg
    }


}

pub struct DirectLocationAnalysis;

impl DirectLocationCPA {
    pub fn new<T: PcodeStore>(pcode: &T) -> Self {
        let info = pcode.info();
        Self {
            cfg: PcodeCfg::new(info),
        }
    }

}
impl ConfigurableProgramAnalysis for DirectLocationCPA {
    type State = PcodeAddressLattice;

    fn reduce(&mut self, state: &Self::State, dest_state: &Self::State, op: &Option<PcodeOperation>) {
        if let PcodeAddressLattice::Value(state) = state {
            self.cfg.add_node(state);
            if let Some(op) = op {
                if let PcodeAddressLattice::Value(dest_state) = dest_state {
                    self.cfg.add_edge(state, dest_state, op.clone());
                }
            }
        }
    }
}

impl Analysis for DirectLocationAnalysis {
    type Output = PcodeCfg<ConcretePcodeAddress, PcodeOperation>;
    type Input = ConcretePcodeAddress;

    fn run<T: PcodeStore, I: Into<Self::Input>>(
        &mut self,
        store: T,
        initial_state: I,
    ) -> Self::Output {
        let initial_state = initial_state.into();
        let lattice = PcodeAddressLattice::Value(initial_state);
        let mut cpa = DirectLocationCPA::new(&store);
        let _ = cpa.run_cpa(lattice, &store);
        cpa.cfg
    }
    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input {
        addr
    }
}
