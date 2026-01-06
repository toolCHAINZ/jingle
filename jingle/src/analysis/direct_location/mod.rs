use crate::analysis::Analysis;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::ConfigurableProgramAnalysis;
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

    pub fn take_cfg(&mut self) -> PcodeCfg<ConcretePcodeAddress, PcodeOperation> {
        let info = self.cfg.info.clone();
        std::mem::replace(&mut self.cfg, PcodeCfg::new(info))
    }

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

impl Analysis for DirectLocationCPA {
    type Output = PcodeCfg<ConcretePcodeAddress, PcodeOperation>;
    type Input = PcodeAddressLattice;

    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input {
        PcodeAddressLattice::Value(addr)
    }

    fn make_output(&mut self, _states: &[Self::State]) -> Self::Output {
        self.take_cfg()
    }
}

// Implement CompoundAnalysis to allow composition with StackOffsetCPA
impl crate::analysis::compound::CompoundAnalysis<crate::analysis::stack_offset::StackOffsetCPA> for DirectLocationCPA {}

