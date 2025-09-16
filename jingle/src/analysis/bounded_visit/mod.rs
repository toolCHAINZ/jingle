mod state;

use crate::analysis::Analysis;
use crate::analysis::bounded_visit::state::BoundedStepsState;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::flat::FlatLattice::Value;
use crate::analysis::cpa::state::AbstractState;
use crate::analysis::direct_location::DirectLocationCPA;
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

struct BoundedStepsCpa<T: PcodeStore> {
    pcode: T,
    cfg: PcodeCfg,
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
    type Iter = Box<dyn Iterator<Item = Self::State>>;

    fn successor_states(&mut self, state: &Self::State) -> Self::Iter {
        let opt = state.location.value().cloned();
        if let Some(addr) = opt {
            self.cfg.add_node(addr);
            let iter = self.pcode.get_pcode_op_at(addr);
            let state = state.clone();
            let i = iter
                .into_iter()
                .flat_map(|op| {
                    let a = state.transfer(&op).inspect(|to| {
                        let op = op.clone();
                        if let Value(a) = to.location {
                            self.cfg.add_edge(addr, a, op)
                        }
                    }).collect::<Vec<_>>();
                    a.into_iter()
                })
                .collect::<Vec<_>>();
            Box::new(i.into_iter())
        } else {
            Box::new(std::iter::empty())
        }
    }
}

pub struct BoundedStepLocationAnalysis {
    max_steps: usize,
}

impl Analysis for BoundedStepLocationAnalysis {
    type Output = PcodeCfg;
    type Input = BoundedStepsState;

    fn run<T: PcodeStore, I: Into<Self::Input>>(
        &mut self,
        store: T,
        initial_state: I,
    ) -> Self::Output {
        let mut cpa = BoundedStepsCpa::new(store);
        let _ =cpa.run_cpa(initial_state.into());
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