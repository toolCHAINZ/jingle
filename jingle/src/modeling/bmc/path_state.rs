use crate::modeling::bmc::address::SymbolicPcodeAddress;
use crate::modeling::bmc::memory_state::MemoryState;

pub struct PcodeVMState<'a, 'ctx, 'sl> {
    memory: MemoryState<'a, 'ctx, 'sl>,
    pc: SymbolicPcodeAddress<'ctx>,
}

impl<'a, 'ctx, 'sl> PcodeVMState<'a, 'ctx, 'sl> {
    pub fn new(memory: MemoryState<'a, 'ctx, 'sl>, pc: SymbolicPcodeAddress<'ctx>) -> Self {
        Self { memory, pc }
    }
}
