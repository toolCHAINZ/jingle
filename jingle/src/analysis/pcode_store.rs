use std::borrow::Borrow;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use jingle_sleigh::{ArchInfoProvider, PcodeOperation, VarNode};

pub trait PcodeStore {
    fn get_pcode_op_at<T: Borrow<ConcretePcodeAddress>>(&self, addr: T) -> Option<PcodeOperation>;
}

pub trait EntryPoint {
    fn get_entry(&self) -> ConcretePcodeAddress;
}

impl<'a> PcodeStore for LoadedSleighContext<'a> {
    fn get_pcode_op_at<T: Borrow<ConcretePcodeAddress>>(&self, addr: T) -> Option<PcodeOperation> {
        let addr = addr.borrow();
        let instr = self.instruction_at(addr.machine())?;
        if addr.pcode() as usize == instr.ops.len() {
            Some(PcodeOperation::Branch {
                input: VarNode {
                    space_index: self.get_code_space_idx(),
                    offset: addr.machine() + instr.length as u64,
                    size: 1,
                },
            })
        } else {
            instr.ops.get(addr.pcode() as usize).cloned()
        }
    }
}

impl<T: PcodeStore> PcodeStore for &T {
    fn get_pcode_op_at<B: Borrow<ConcretePcodeAddress>>(&self, addr: B) -> Option<PcodeOperation> {
        (*self).get_pcode_op_at(addr)
    }
}
