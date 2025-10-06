use crate::analysis::Analysis;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;

pub struct DirectLocationCPA<T> {
    pcode: T,
    cfg: PcodeCfg<ConcretePcodeAddress, PcodeOperation>,
}

impl<T> DirectLocationCPA<T> {
    pub fn cfg(&self) -> &PcodeCfg<ConcretePcodeAddress, PcodeOperation> {
        &self.cfg
    }
}

pub struct DirectLocationAnalysis;

impl<T: PcodeStore> DirectLocationCPA<T> {
    pub fn new(pcode: T) -> Self {
        Self {
            pcode,
            cfg: Default::default(),
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

    fn get_pcode_store(&self) -> &impl PcodeStore {
        &self.pcode
    }

    fn reduce(&mut self, state: &Self::State, dest_state: &Self::State) {
        if let PcodeAddressLattice::Value(state) = state {
            self.cfg.add_node(state);
            if let Some(op) = self.pcode.get_pcode_op_at(state) {
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
        let mut cpa = DirectLocationCPA::new(store);
        let _ = cpa.run_cpa(lattice);
        cpa.cfg
    }
    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input {
        addr
    }
}
