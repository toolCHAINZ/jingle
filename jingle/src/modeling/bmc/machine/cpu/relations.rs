use jingle_sleigh::PcodeOperation;
use crate::JingleError;
use crate::modeling::bmc::machine::cpu::SymbolicPcodeAddress;
use crate::modeling::bmc::machine::memory::MemoryState;

impl<'ctx> SymbolicPcodeAddress<'ctx>{
    pub(crate) fn apply_op(&self, memory: &MemoryState<'_, 'ctx, '_>, op: &PcodeOperation) -> Result<Self, JingleError>{
        match op{
            _=> {
                todo!()
            }
        }
    }
}