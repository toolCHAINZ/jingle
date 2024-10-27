use crate::modeling::bmc::address::SymbolicPcodeAddress;
use crate::modeling::bmc::state::MemoryState;

pub struct FlowState<'a, 'ctx, 'sl> {
    memory: MemoryState<'a, 'ctx, 'sl>,
    pc: SymbolicPcodeAddress<'ctx>,
}

impl<'a, 'ctx, 'sl> FlowState<'a, 'ctx, 'sl> {
    pub fn new(memory: MemoryState<'a, 'ctx, 'sl>, pc: SymbolicPcodeAddress<'ctx>) -> Self {
        Self { memory, pc }
    }
}
