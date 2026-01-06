use crate::analysis::Analysis;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::iter::{empty, once};

/// How this analysis treats direct call instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CallBehavior {
    /// Follow it like any other branch
    Branch,
    /// Treat it as a no-op (and eventually model function summaries in the step over)
    StepOver,
    /// Terminate this path
    Terminate,
}

/// State wrapper that customizes call handling
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DirectLocationState {
    inner: PcodeAddressLattice,
    call_behavior: CallBehavior,
}

impl DirectLocationState {
    pub fn new(addr: ConcretePcodeAddress, call_behavior: CallBehavior) -> Self {
        Self {
            inner: PcodeAddressLattice::Value(addr),
            call_behavior,
        }
    }

    pub fn top(call_behavior: CallBehavior) -> Self {
        Self {
            inner: PcodeAddressLattice::Top,
            call_behavior,
        }
    }

    pub fn inner(&self) -> &PcodeAddressLattice {
        &self.inner
    }
}

impl From<DirectLocationState> for PcodeAddressLattice {
    fn from(state: DirectLocationState) -> Self {
        state.inner
    }
}

impl PartialOrd for DirectLocationState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.call_behavior != other.call_behavior {
            None
        } else {
            self.inner.partial_cmp(&other.inner)
        }
    }
}

impl Ord for DirectLocationState {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl JoinSemiLattice for DirectLocationState {
    fn join(&mut self, other: &Self) {
        self.inner.join(&other.inner);
    }
}

impl AbstractState for DirectLocationState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.inner.merge(&other.inner)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.inner.stop(states.map(|s| &s.inner))
    }

    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, op: B) -> Successor<'a, Self> {
        let op = op.borrow();

        // Custom handling for Call operations based on call_behavior
        if let PcodeOperation::Call { dest, .. } = op {
            match self.call_behavior {
                CallBehavior::Branch => {
                    // Follow the call like a branch
                    if let PcodeAddressLattice::Value(_addr) = &self.inner {
                        let call_target = ConcretePcodeAddress::from(dest.offset);
                        return once(DirectLocationState::new(call_target, self.call_behavior)).into();
                    }
                }
                CallBehavior::StepOver => {
                    // Fall through to next instruction
                    if let PcodeAddressLattice::Value(addr) = &self.inner {
                        let next = addr.next_pcode();
                        return once(DirectLocationState::new(next, self.call_behavior)).into();
                    }
                }
                CallBehavior::Terminate => {
                    // Terminate this path
                    return empty().into();
                }
            }
        }

        // Default behavior: delegate to inner state and wrap results
        match &self.inner {
            PcodeAddressLattice::Value(addr) => {
                addr.transfer(op)
                    .into_iter()
                    .map(|next_addr| DirectLocationState::new(next_addr, self.call_behavior))
                    .into()
            }
            PcodeAddressLattice::Top => once(DirectLocationState::top(self.call_behavior)).into(),
        }
    }
}

impl LocationState for DirectLocationState {
    fn get_operation<T: PcodeStore>(&self, t: &T) -> Option<PcodeOperation> {
        self.inner.get_operation(t)
    }
}

impl crate::analysis::compound::Strengthen<crate::analysis::direct_valuation::DirectValuationState> for DirectLocationState {}

pub struct DirectLocationAnalysis {
    cfg: PcodeCfg<ConcretePcodeAddress, PcodeOperation>,
    call_behavior: CallBehavior,
}

impl DirectLocationAnalysis {
    pub fn cfg(&self) -> &PcodeCfg<ConcretePcodeAddress, PcodeOperation> {
        &self.cfg
    }

    pub fn take_cfg(&mut self) -> PcodeCfg<ConcretePcodeAddress, PcodeOperation> {
        let info = self.cfg.info.clone();
        std::mem::replace(&mut self.cfg, PcodeCfg::new(info))
    }

    pub fn call_behavior(&self) -> CallBehavior {
        self.call_behavior
    }

    pub fn set_call_behavior(&mut self, behavior: CallBehavior) {
        self.call_behavior = behavior;
    }

    pub fn new<T: PcodeStore>(pcode: &T) -> Self {
        Self::with_call_behavior(pcode, CallBehavior::StepOver)
    }

    pub fn with_call_behavior<T: PcodeStore>(pcode: &T, call_behavior: CallBehavior) -> Self {
        let info = pcode.info();
        Self {
            cfg: PcodeCfg::new(info),
            call_behavior,
        }
    }
}

impl ConfigurableProgramAnalysis for DirectLocationAnalysis {
    type State = DirectLocationState;

    fn reduce(&mut self, state: &Self::State, dest_state: &Self::State, op: &Option<PcodeOperation>) {
        if let PcodeAddressLattice::Value(state_addr) = &state.inner {
            self.cfg.add_node(state_addr);
            if let Some(op) = op {
                if let PcodeAddressLattice::Value(dest_addr) = &dest_state.inner {
                    self.cfg.add_edge(state_addr, dest_addr, op.clone());
                }
            }
        }
    }
}

impl Analysis for DirectLocationAnalysis {
    type Input = DirectLocationState;

    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input {
        DirectLocationState::new(addr, self.call_behavior)
    }

    // Default implementation: just returns the states
    // To access the built CFG, use .cfg() or .take_cfg() on the analysis instance
}

// Enable compound analysis: DirectLocationAnalysis can be strengthened by DirectValuationAnalysis
impl crate::analysis::compound::CompoundAnalysis<crate::analysis::direct_valuation::DirectValuationAnalysis> for DirectLocationAnalysis {}

#[cfg(test)]
mod tests {
    use super::*;
    use jingle_sleigh::VarNode;

    #[test]
    fn test_call_behavior_branch() {
        let state = DirectLocationState::new(
            ConcretePcodeAddress::from(0x1000),
            CallBehavior::Branch,
        );

        let call_op = PcodeOperation::Call {
            dest: VarNode {
                space_index: 0,
                offset: 0x2000,
                size: 8,
            },
            args: vec![],
            call_info: None,
        };

        let successors: Vec<_> = state.transfer(&call_op).into_iter().collect();
        assert_eq!(successors.len(), 1);
        assert_eq!(successors[0].inner, PcodeAddressLattice::Value(ConcretePcodeAddress::from(0x2000)));
    }

    #[test]
    fn test_call_behavior_step_over() {
        let state = DirectLocationState::new(
            ConcretePcodeAddress::from(0x1000),
            CallBehavior::StepOver,
        );

        let call_op = PcodeOperation::Call {
            dest: VarNode {
                space_index: 0,
                offset: 0x2000,
                size: 8,
            },
            args: vec![],
            call_info: None,
        };

        let successors: Vec<_> = state.transfer(&call_op).into_iter().collect();
        assert_eq!(successors.len(), 1);
        // Should step over to next pcode address (machine: 0x1000, pcode: 1)
        let expected = ConcretePcodeAddress::from(0x1000).next_pcode();
        assert_eq!(successors[0].inner, PcodeAddressLattice::Value(expected));
    }

    #[test]
    fn test_call_behavior_terminate() {
        let state = DirectLocationState::new(
            ConcretePcodeAddress::from(0x1000),
            CallBehavior::Terminate,
        );

        let call_op = PcodeOperation::Call {
            dest: VarNode {
                space_index: 0,
                offset: 0x2000,
                size: 8,
            },
            args: vec![],
            call_info: None,
        };

        let successors: Vec<_> = state.transfer(&call_op).into_iter().collect();
        assert_eq!(successors.len(), 0, "Terminate should produce no successors");
    }
}
