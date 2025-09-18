use crate::JingleError;
use crate::modeling::concretize::{ConcretizationIterator, Concretize};
use crate::modeling::machine::memory::MemoryState;
use cpu::concrete::ConcretePcodeAddress;
use cpu::concrete::PcodeMachineAddress;
use cpu::symbolic::SymbolicPcodeAddress;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use std::borrow::Borrow;
use z3::ast::Bool;

pub mod cpu;
pub mod memory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineState {
    pub jingle: SleighArchInfo,
    memory: MemoryState,
    pc: SymbolicPcodeAddress,
}

impl MachineState {
    pub fn fresh<T: Borrow<SleighArchInfo>>(jingle: T) -> Self {
        Self {
            jingle: jingle.borrow().clone(),
            memory: MemoryState::fresh(jingle),
            pc: SymbolicPcodeAddress::fresh(),
        }
    }

    pub fn fresh_for_machine_address<T: Borrow<SleighArchInfo>>(
        jingle: T,
        machine_addr: PcodeMachineAddress,
    ) -> Self {
        let pc = ConcretePcodeAddress::from(machine_addr);
        Self {
            jingle: jingle.borrow().clone(),
            memory: MemoryState::fresh_for_address(jingle, pc),
            pc: pc.symbolize(),
        }
    }

    pub fn fresh_for_address<T: Borrow<ConcretePcodeAddress>, S: Borrow<SleighArchInfo>>(
        jingle: S,
        addr: T,
    ) -> Self {
        let addr = addr.borrow();
        Self {
            jingle: jingle.borrow().clone(),
            memory: MemoryState::fresh_for_address(jingle, addr),
            pc: addr.symbolize(),
        }
    }

    pub fn concretize_with_assertions<T: Concretize, I: Iterator<Item = Bool>>(
        &self,
        t: &T,
        a: I,
    ) -> ConcretizationIterator<T> {
        ConcretizationIterator::new_with_assertions(a, t)
    }

    fn apply_control_flow(&self, op: &PcodeOperation) -> Result<SymbolicPcodeAddress, JingleError> {
        self.pc.apply_op(&self.memory, op)
    }

    pub fn apply(&self, op: &PcodeOperation) -> Result<Self, JingleError> {
        Ok(Self {
            jingle: self.jingle.clone(),
            memory: self.memory.apply(op)?,
            pc: self.apply_control_flow(op)?,
        })
    }

    pub fn eq(&self, other: &Self) -> Bool {
        let machine_eq = self.pc.machine.eq(&other.pc.machine);
        self.pc.eq(&other.pc) & self.memory._eq(&other.memory, &machine_eq)
    }

    pub fn pc(&self) -> &SymbolicPcodeAddress {
        &self.pc
    }

    pub fn memory(&self) -> &MemoryState {
        &self.memory
    }
}
