mod state;

use crate::analysis::Analysis;
use crate::analysis::bounded_visit::state::BoundedStepsState;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::lattice::flat::FlatLattice::{self, Value};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;

pub struct BoundedStepsCpa {
    cfg: PcodeCfg<ConcretePcodeAddress, PcodeOperation>,
    max_steps: usize,
}

impl BoundedStepsCpa {
    pub fn new<T: PcodeStore>(pcode: &T, max_steps: usize) -> Self {
        let info = pcode.info();
        Self {
            cfg: PcodeCfg::new(info),
            max_steps,
        }
    }

    pub fn take_cfg(&mut self) -> PcodeCfg<ConcretePcodeAddress, PcodeOperation> {
        let info = self.cfg.info.clone();
        std::mem::replace(&mut self.cfg, PcodeCfg::new(info))
    }

    pub fn new_with_max_steps<T: PcodeStore>(pcode: &T, max_steps: usize) -> Self {
        Self::new(pcode, max_steps)
    }

    /// Inherent constructor for the analysis initial state.
    ///
    /// The `Analysis` trait no longer exposes an associated `Input` or a
    /// `make_initial_state` method. Provide an inherent helper so callers can
    /// construct the appropriate initial `BoundedStepsState` using the analysis
    /// instance (for access to `max_steps`).
    pub fn make_initial_state(&self, addr: ConcretePcodeAddress) -> BoundedStepsState {
        BoundedStepsState::new(addr.into(), self.max_steps)
    }
}

impl ConfigurableProgramAnalysis for BoundedStepsCpa {
    type State = BoundedStepsState;

    fn reduce(
        &mut self,
        state: &Self::State,
        dest_state: &Self::State,
        op: &Option<PcodeOperation>,
    ) {
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

impl IntoState<BoundedStepsCpa> for ConcretePcodeAddress {
    fn into_state(self, c: &BoundedStepsCpa) -> BoundedStepsState {
        BoundedStepsState::new(FlatLattice::Value(self), c.max_steps)
    }
}

impl Analysis for BoundedStepsCpa {}

pub type BoundedStepLocationAnalysis = BoundedStepsCpa;
