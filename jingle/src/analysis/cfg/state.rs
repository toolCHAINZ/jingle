use crate::JingleError;
use crate::modeling::machine::MachineState;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use std::fmt::Debug;
use std::hash::Hash;
use z3::ast::Bool;

pub trait CfgStateModel: Sized {
    fn location_eq(&self, other: &Self) -> Bool;

    fn mem_eq(&self, other: &Self) -> Bool;

    fn apply(&self, op: &PcodeOperation) -> Result<Self, JingleError>;
}

impl CfgStateModel for MachineState {
    fn location_eq(&self, other: &Self) -> Bool {
        self.pc().eq(other.pc())
    }

    fn mem_eq(&self, other: &Self) -> Bool {
        let machine_eq = self.pc().machine.eq(&other.pc().machine);
        self.memory()._eq(other.memory(), &machine_eq)
    }

    fn apply(&self, op: &PcodeOperation) -> Result<Self, JingleError> {
        self.apply(op)
    }
}

pub trait CfgState: Clone + Debug + Hash + Eq {
    type Model: CfgStateModel + Clone;

    fn fresh(&self, i: &SleighArchInfo) -> Self::Model;
}

pub trait ModelTransition<S: CfgState>: Clone {
    fn transition(&self, init: &S::Model) -> Result<S::Model, JingleError>;
}

impl CfgState for ConcretePcodeAddress {
    type Model = MachineState;

    fn fresh(&self, i: &SleighArchInfo) -> Self::Model {
        MachineState::fresh_for_address(i, *self)
    }
}

impl<N: CfgState> ModelTransition<N> for PcodeOperation {
    fn transition(&self, init: &N::Model) -> Result<N::Model, JingleError> {
        init.apply(self)
    }
}

impl<N: CfgState, T: ModelTransition<N>> ModelTransition<N> for Vec<T> {
    fn transition(&self, init: &N::Model) -> Result<N::Model, JingleError> {
        let mut state = init.clone();
        for op in self {
            state = op.transition(&state)?;
        }
        Ok(state)
    }
}
