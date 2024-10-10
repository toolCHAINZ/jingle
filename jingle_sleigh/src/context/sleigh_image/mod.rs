mod instruction_iterator;

use crate::context::sleigh_image::instruction_iterator::SleighContextInstructionIterator;
use crate::ffi::sleigh_image::bridge::SleighImage as SleighImageFFI;
use crate::{Instruction, RegisterManager, SpaceInfo, SpaceManager, VarNode};
use cxx::UniquePtr;
use std::ops::Index;
use crate::context::SleighContext;
use crate::ffi::instruction::bridge::VarnodeInfoFFI;

pub struct SleighImage {
    img_ffi: UniquePtr<SleighImageFFI>,
    spaces: Vec<SpaceInfo>,

}

impl SleighImage {
    pub(crate) fn new(ffi: UniquePtr<SleighImageFFI>) -> Self {
        let mut spaces: Vec<SpaceInfo> = Vec::with_capacity(ffi.getNumSpaces() as usize);
        for idx in 0..ffi.getNumSpaces() {
            spaces.push(SpaceInfo::from(ffi.getSpaceByIndex(idx)));
        }

        Self { img_ffi: ffi, spaces }
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

impl SpaceManager for SleighImage {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.spaces.get(idx)
    }

    fn get_all_space_info(&self) -> &[SpaceInfo] {
        self.spaces.as_slice()
    }

    fn get_code_space_idx(&self) -> usize {
        self.img_ffi
            .getSpaceByIndex(0)
            .getManager()
            .getDefaultCodeSpace()
            .getIndex() as usize
    }
}

impl RegisterManager for SleighImage {
    fn get_register(&self, name: &str) -> Option<VarNode> {
        self.img_ffi.getRegister(name).map(VarNode::from).ok()
    }

    fn get_register_name(&self, location: VarNode) -> Option<&str> {
        let space = self.img_ffi.getSpaceByIndex(location.space_index as i32);
        self.img_ffi
            .getRegisterName(VarnodeInfoFFI {
                space,
                offset: location.offset,
                size: location.size,
            })
            .ok()
    }

    fn get_registers(&self) -> Vec<(VarNode, String)> {
        self.img_ffi
            .getRegisters()
            .iter()
            .map(|b| (VarNode::from(&b.varnode), b.name.clone()))
            .collect()
    }
}

#[cfg(test)]
mod test{
    use crate::context::SleighContextBuilder;
    use crate::tests::SLEIGH_ARCH;

    #[test]
    fn test_two_images(){
        let mov_eax_0: [u8; 4] = [0x0f, 0x05, 0x0f, 0x05];
        let nops: [u8; 4] = [0x90, 0x90, 0x90, 0x90];
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder
            .build(SLEIGH_ARCH)
            .unwrap();
       let img1 = sleigh.load_image(mov_eax_0.as_slice()).unwrap();
       let img2 = sleigh.load_image(nops.as_slice()).unwrap();
       let instr1 = img1.instruction_at(0);
       let instr2 = img2.instruction_at(0);
       assert_eq!(instr1, instr2);
       assert_ne!(instr1, None);
    }
}