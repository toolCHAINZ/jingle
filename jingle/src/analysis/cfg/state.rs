use std::hash::Hash;
use z3::ast::Bool;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use crate::JingleError;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use crate::modeling::machine::MachineState;

pub trait CfgStateModel {
    fn location_eq(&self, other: &Self) -> Bool;
    fn eq(&self, other: &Self) -> Bool;
}

impl CfgStateModel for MachineState {
    fn location_eq(&self, other: &Self) -> Bool {
        self.pc().eq(other.pc())
    }

    fn eq(&self, other: &Self) -> Bool {
        self.eq(other)
    }
}

pub trait CfgState: Clone + Hash + Eq {
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

impl ModelTransition<ConcretePcodeAddress> for PcodeOperation {
    fn transition(&self, init: &MachineState) -> Result<MachineState, JingleError> {
        init.apply(self)
    }
}