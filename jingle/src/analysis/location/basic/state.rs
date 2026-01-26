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
            state::{AbstractState, LocationState, MergeOutcome, StateDisplay, Successor},
        },
        direct_valuation::DirectValuationState,
        direct_valuation2::DirectValuation2State,
        location::basic::DirectLocationAnalysis,
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

impl Display for DirectLocationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // PcodeAddressLattice doesn't implement Display, so we use Debug
        write!(f, "{:?}", self.inner)
    }
}

impl LowerHex for DirectLocationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        LowerHex::fmt(&self.inner, f)
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

impl StateDisplay for DirectLocationState {
    fn fmt_state(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Use LowerHex format for the inner PcodeAddressLattice
        write!(f, "{:x}", self.inner)
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
        self.inner
            .transfer(op)
            .into_iter()
            .map(|next_addr| DirectLocationState::new(next_addr, self.call_behavior))
            .into()
    }
}

impl LocationState for DirectLocationState {
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

impl DirectLocationState {
    pub fn strengthen_from_valuation2(&self, _v: &DirectValuation2State) -> Option<Self> {
        todo!()
    }

    pub fn strengthen_from_valuation(&self, v: &DirectValuationState) -> Option<Self> {
        dbg!(self, v);
        None
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

register_strengthen!(
    DirectLocationState,
    DirectValuation2State,
    DirectLocationState::strengthen_from_valuation2
);

register_strengthen!(
    DirectLocationState,
    DirectValuationState,
    DirectLocationState::strengthen_from_valuation
);
