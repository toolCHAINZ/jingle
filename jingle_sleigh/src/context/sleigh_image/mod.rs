mod instruction_iterator;

use crate::context::sleigh_image::instruction_iterator::SleighContextInstructionIterator;
use crate::ffi::sleigh_image::bridge::SleighImage as SleighImageFFI;
use crate::Instruction;
use cxx::UniquePtr;
use std::ops::Index;

pub struct SleighImage {
    img_ffi: UniquePtr<SleighImageFFI>,
}

impl SleighImage {
    pub(crate) fn new(ffi: UniquePtr<SleighImageFFI>) -> Self {
        Self { img_ffi: ffi }
    }

    pub fn instruction_at(&self, offset: u64) -> Option<Instruction> {
        self.img_ffi
            .get_one_instruction(offset)
            .map(Instruction::from)
            .ok()
    }
    pub fn read(&self, offset: u64, max_instrs: usize) -> SleighContextInstructionIterator {
        SleighContextInstructionIterator::new(self, offset, max_instrs, false)
    }

    pub fn read_until_branch(
        &self,
        offset: u64,
        max_instrs: usize,
    ) -> SleighContextInstructionIterator {
        SleighContextInstructionIterator::new(self, offset, max_instrs, true)
    }
}
