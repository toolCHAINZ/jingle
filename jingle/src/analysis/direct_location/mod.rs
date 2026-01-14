use crate::analysis::Analysis;
use crate::analysis::bounded_branch::state::BoundedBranchState;
use crate::analysis::cfg::{CfgState, PcodeCfg};
use crate::analysis::compound::{Strengthen, StrengthenOutcome};
use crate::analysis::cpa::lattice::JoinSemiLattice;

use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::reducer::CfgReducer;
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::MachineState;
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
    pub fn new(addr: PcodeAddressLattice, call_behavior: CallBehavior) -> Self {
        Self {
            inner: addr,
            call_behavior,
        }
    }

    pub fn location(addr: ConcretePcodeAddress, call_behavior: CallBehavior) -> Self {
        Self {
            inner: PcodeAddressLattice::Const(addr),
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

impl IntoState<DirectLocationAnalysis> for ConcretePcodeAddress {
    fn into_state(
        self,
        c: &DirectLocationAnalysis,
    ) -> <DirectLocationAnalysis as ConfigurableProgramAnalysis>::State {
        DirectLocationState {
            call_behavior: c.call_behavior,
            inner: PcodeAddressLattice::Const(self),
        }
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
                    if let PcodeAddressLattice::Const(_addr) = &self.inner {
                        let call_target = ConcretePcodeAddress::from(dest.offset);
                        return once(DirectLocationState::location(
                            call_target,
                            self.call_behavior,
                        ))
                        .into();
                    }
                }
                CallBehavior::StepOver => {
                    // Fall through to next instruction
                    if let PcodeAddressLattice::Const(addr) = &self.inner {
                        let next = addr.next_pcode();
                        return once(DirectLocationState::location(next, self.call_behavior))
                            .into();
                    }
                }
                CallBehavior::Terminate => {
                    // Terminate this path
                    return empty().into();
                }
            }
        }

        // Default behavior: delegate to inner state and wrap results
        return self
            .inner
            .transfer(op)
            .into_iter()
            .map(|next_addr| DirectLocationState::new(next_addr, self.call_behavior))
            .into();
    }
}

impl LocationState for DirectLocationState {
    fn get_operation<T: PcodeStore>(&self, t: &T) -> Option<PcodeOperation> {
        self.inner.get_operation(t)
    }

    fn get_location(&self) -> Option<ConcretePcodeAddress> {
        self.inner.value().cloned()
    }
}

impl crate::analysis::compound::Strengthen<crate::analysis::direct_valuation::DirectValuationState>
    for DirectLocationState
{
}

impl Strengthen<BoundedBranchState> for DirectLocationState {
    fn strengthen(
        &mut self,
        _original: &(Self, BoundedBranchState),
        _other: &BoundedBranchState,
        _op: &PcodeOperation,
    ) -> StrengthenOutcome {
        // DirectLocationState does not gain any additional information from the
        // BoundedBranchState, so leave it unchanged.
        StrengthenOutcome::Unchanged
    }
}

pub struct DirectLocationAnalysis {
    call_behavior: CallBehavior,
}

impl DirectLocationAnalysis {
    pub fn call_behavior(&self) -> CallBehavior {
        self.call_behavior
    }

    pub fn set_call_behavior(&mut self, behavior: CallBehavior) {
        self.call_behavior = behavior;
    }

    pub fn new(call_behavior: CallBehavior) -> Self {
        Self { call_behavior }
    }
}

impl CfgState for DirectLocationState {
    type Model = MachineState;

    fn new_const(&self, i: &jingle_sleigh::SleighArchInfo) -> Self::Model {
        match &self.inner {
            PcodeAddressLattice::Const(addr) => MachineState::fresh_for_address(i, *addr),
            // For computed or unknown locations, fall back to a generic fresh machine state.
            PcodeAddressLattice::Computed(_) | PcodeAddressLattice::Top => MachineState::fresh(i),
        }
    }

    fn model_id(&self) -> String {
        match &self.inner {
            PcodeAddressLattice::Const(a) => a.model_id(),
            PcodeAddressLattice::Top => "State_Top_".to_string(),
            PcodeAddressLattice::Computed(_) => "State_Computed_".to_string(),
        }
    }

    fn location(&self) -> Option<ConcretePcodeAddress> {
        self.inner.value().cloned()
    }
}

impl ConfigurableProgramAnalysis for DirectLocationAnalysis {
    type State = DirectLocationState;
    type Reducer = CfgReducer<Self::State>;
}

impl Analysis for DirectLocationAnalysis {}

// Enable compound analysis: DirectLocationAnalysis can be strengthened by DirectValuationAnalysis
impl
    crate::analysis::compound::CompoundAnalysis<
        crate::analysis::direct_valuation::DirectValuationAnalysis,
    > for DirectLocationAnalysis
{
}

// Enable compound analysis: DirectLocationAnalysis can be strengthened by BoundedBranchAnalysis
impl
    crate::analysis::compound::CompoundAnalysis<
        crate::analysis::bounded_branch::BoundedBranchAnalysis,
    > for DirectLocationAnalysis
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use jingle_sleigh::VarNode;

    #[test]
    fn test_call_behavior_branch() {
        let state =
            DirectLocationState::location(ConcretePcodeAddress::from(0x1000), CallBehavior::Branch);

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
        assert_eq!(
            successors[0].inner,
            PcodeAddressLattice::Const(ConcretePcodeAddress::from(0x2000))
        );
    }

    #[test]
    fn test_call_behavior_step_over() {
        let state = DirectLocationState::location(
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
        assert_eq!(successors[0].inner, PcodeAddressLattice::Const(expected));
    }

    #[test]
    fn test_call_behavior_terminate() {
        let state = DirectLocationState::location(
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
        assert_eq!(
            successors.len(),
            0,
            "Terminate should produce no successors"
        );
    }
}
