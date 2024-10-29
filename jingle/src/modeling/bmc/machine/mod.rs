use crate::modeling::bmc::machine::memory::MemoryState;
use cpu::SymbolicPcodeAddress;

pub(crate) mod cpu;
pub(crate) mod memory;
pub struct MachineState<'a, 'ctx, 'sl> {
    memory: MemoryState<'a, 'ctx, 'sl>,
    pc: SymbolicPcodeAddress<'ctx>,
}

impl<'a, 'ctx, 'sl> MachineState<'a, 'ctx, 'sl> {
    pub fn new(memory: MemoryState<'a, 'ctx, 'sl>, pc: SymbolicPcodeAddress<'ctx>) -> Self {
        Self { memory, pc }
    }
}
