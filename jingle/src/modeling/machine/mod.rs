use crate::modeling::machine::memory::MemoryState;
use crate::{JingleContext, JingleError};
use cpu::concrete::ConcretePcodeAddress;
use cpu::concrete::PcodeMachineAddress;
use cpu::symbolic::SymbolicPcodeAddress;
use jingle_sleigh::PcodeOperation;
use z3::ast::Bool;

pub mod cpu;
pub mod memory;

#[derive(Debug, Clone)]
pub struct MachineState<'ctx> {
    jingle: JingleContext<'ctx>,
    memory: MemoryState<'ctx>,
    pc: SymbolicPcodeAddress<'ctx>,
}

impl<'ctx> MachineState<'ctx> {
    pub fn fresh(jingle: &JingleContext<'ctx>) -> Self {
        Self {
            jingle: jingle.clone(),
            memory: MemoryState::fresh(jingle),
            pc: SymbolicPcodeAddress::fresh(jingle.z3),
        }
    }

    pub fn fresh_for_machine_address(
        jingle: &JingleContext<'ctx>,
        machine_addr: PcodeMachineAddress,
    ) -> Self {
        let pc = ConcretePcodeAddress::from(machine_addr);
        Self {
            jingle: jingle.clone(),
            memory: MemoryState::fresh(jingle),
            pc: pc.symbolize(jingle.z3),
        }
    }

    pub fn fresh_for_address(jingle: &JingleContext<'ctx>, addr: ConcretePcodeAddress) -> Self {
        Self {
            jingle: jingle.clone(),
            memory: MemoryState::fresh(jingle),
            pc: addr.symbolize(jingle.z3),
        }
    }

    fn apply_control_flow(
        &self,
        op: &PcodeOperation,
    ) -> Result<SymbolicPcodeAddress<'ctx>, JingleError> {
        self.pc.apply_op(&self.memory, op, self.jingle.z3)
    }

    pub fn apply(&self, op: &PcodeOperation) -> Result<Self, JingleError> {
        Ok(Self {
            jingle: self.jingle.clone(),
            memory: self.memory.apply(op)?,
            pc: self.apply_control_flow(op)?,
        })
    }

    pub fn _eq(&self, other: &Self) -> Bool<'ctx> {
        self.pc._eq(&other.pc) & self.memory._eq(&other.memory)
    }

    pub fn pc(&self) -> &SymbolicPcodeAddress<'ctx> {
        &self.pc
    }

    pub fn memory(&self) -> &MemoryState<'ctx> {
        &self.memory
    }
}
