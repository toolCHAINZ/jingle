use std::{
    borrow::Borrow,
    cmp::Ordering,
    fmt::{Display, LowerHex},
    iter::{empty, once},
};

use jingle_sleigh::PcodeOperation;

use crate::{
    analysis::{
        cfg::CfgState,
        cpa::{
            ConfigurableProgramAnalysis, IntoState,
            lattice::{JoinSemiLattice, pcode::PcodeAddressLattice},
            state::{AbstractState, LocationState, MergeOutcome, Successor},
        },
        location::basic::BasicLocationAnalysis,
        valuation::SimpleValuationState,
    },
    modeling::machine::{MachineState, cpu::concrete::ConcretePcodeAddress},
    register_strengthen,
};

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
pub struct BasicLocationState {
    inner: PcodeAddressLattice,
    call_behavior: CallBehavior,
}

impl BasicLocationState {
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

impl IntoState<BasicLocationAnalysis> for ConcretePcodeAddress {
    fn into_state(
        self,
        c: &BasicLocationAnalysis,
    ) -> <BasicLocationAnalysis as ConfigurableProgramAnalysis>::State {
        BasicLocationState {
            call_behavior: c.call_behavior,
            inner: PcodeAddressLattice::Const(self),
        }
    }
}

impl From<BasicLocationState> for PcodeAddressLattice {
    fn from(state: BasicLocationState) -> Self {
        state.inner
    }
}

impl Display for BasicLocationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // PcodeAddressLattice doesn't implement Display, so we use Debug
        write!(f, "{}", self.inner)
    }
}

impl LowerHex for BasicLocationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        LowerHex::fmt(&self.inner, f)
    }
}

impl PartialOrd for BasicLocationState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.call_behavior != other.call_behavior {
            None
        } else {
            self.inner.partial_cmp(&other.inner)
        }
    }
}

impl JoinSemiLattice for BasicLocationState {
    fn join(&mut self, other: &Self) {
        self.inner.join(&other.inner);
    }
}

impl AbstractState for BasicLocationState {
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
                        return once(BasicLocationState::location(
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
                        return once(BasicLocationState::location(next, self.call_behavior)).into();
                    }
                }
                CallBehavior::Terminate => {
                    // Terminate this path
                    return empty().into();
                }
            }
        }

        // Default behavior: delegate to inner state and wrap results
        self.inner
            .transfer(op)
            .into_iter()
            .map(|next_addr| BasicLocationState::new(next_addr, self.call_behavior))
            .into()
    }
}

impl LocationState for BasicLocationState {
    fn get_operation<'a, T: crate::analysis::pcode_store::PcodeStore + ?Sized>(
        &'a self,
        t: &'a T,
    ) -> Option<crate::analysis::pcode_store::PcodeOpRef<'a>> {
        self.inner.get_operation(t)
    }

    fn get_location(&self) -> Option<ConcretePcodeAddress> {
        self.inner.value().cloned()
    }
}

impl BasicLocationState {
    pub fn strengthen_from_valuation(&mut self, v: &SimpleValuationState) {
        if let PcodeAddressLattice::Computed(indirect_var_node) = &self.inner {
            let ptr_value = v.get_value(&indirect_var_node.pointer_location);
            if let Some(value) = ptr_value {
                if let Some(v) = value.as_const() {
                    self.inner = PcodeAddressLattice::Const(v.into())
                }
            }
        }
    }
}

impl CfgState for BasicLocationState {
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

register_strengthen!(
    BasicLocationState,
    SimpleValuationState,
    BasicLocationState::strengthen_from_valuation
);
