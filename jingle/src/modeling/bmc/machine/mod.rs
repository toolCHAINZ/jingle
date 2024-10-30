use crate::modeling::bmc::machine::memory::MemoryState;
use cpu::SymbolicPcodeAddress;

pub(crate) mod cpu;
pub(crate) mod memory;
pub struct MachineState<'ctx, 'sl> {
    memory: MemoryState<'ctx, 'sl>,
    pc: SymbolicPcodeAddress<'ctx>,
}

impl<'ctx, 'sl> MachineState<'ctx, 'sl> {
    pub fn new(memory: MemoryState<'ctx, 'sl>, pc: SymbolicPcodeAddress<'ctx>) -> Self {
        Self { memory, pc }
    }
}
