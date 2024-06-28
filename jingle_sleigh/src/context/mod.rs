mod builder;

use crate::error::JingleSleighError;
use crate::error::JingleSleighError::{LanguageSpecRead, SleighInitError};
use crate::ffi::addrspace::bridge::AddrSpaceHandle;
use crate::ffi::context_ffi::bridge::ContextFFI;
use crate::instruction::Instruction;
use crate::space::{RegisterManager, SpaceInfo, SpaceManager};
#[cfg(feature = "gimli")]
pub use builder::image::gimli::map_gimli_architecture;
pub use builder::image::{Image, ImageSection};
pub use builder::SleighContextBuilder;

use crate::context::builder::language_def::LanguageDefinition;
use crate::ffi::context_ffi::CTX_BUILD_MUTEX;
use crate::ffi::instruction::bridge::VarnodeInfoFFI;
use crate::VarNode;
use cxx::{SharedPtr, UniquePtr};
use std::fmt::{Debug, Formatter};
use std::path::Path;

pub struct SleighContext {
    ctx: UniquePtr<ContextFFI>,
    spaces: Vec<SpaceInfo>,
    language_id: String,
    pub image: Image,
}

impl Debug for SleighContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sleigh {{image: {:?}}}", self.image)
    }
}

impl SpaceManager for SleighContext {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.spaces.get(idx)
    }

    fn get_all_space_info(&self) -> &[SpaceInfo] {
        self.spaces.as_slice()
    }

    fn get_code_space_idx(&self) -> usize {
        self.ctx
            .getSpaceByIndex(0)
            .getManager()
            .getDefaultCodeSpace()
            .getIndex() as usize
    }
}

impl RegisterManager for SleighContext {
    fn get_register(&self, name: &str) -> Option<VarNode> {
        self.ctx.getRegister(name).map(VarNode::from).ok()
    }

    fn get_register_name(&self, location: VarNode) -> Option<&str> {
        let space = self.ctx.getSpaceByIndex(location.space_index as i32);
        self.ctx
            .getRegisterName(VarnodeInfoFFI {
                space,
                offset: location.offset,
                size: location.size,
            })
            .ok()
    }

    fn get_registers(&self) -> Vec<(VarNode, String)> {
        self.ctx
            .getRegisters()
            .iter()
            .map(|b| (VarNode::from(&b.varnode), b.name.clone()))
            .collect()
    }
}

impl SleighContext {
    pub(crate) fn new<T: AsRef<Path>>(
        language_def: &LanguageDefinition,
        base_path: T,
        image: Image,
    ) -> Result<Self, JingleSleighError> {
        let path = base_path.as_ref().join(&language_def.sla_file);
        let abs = path.canonicalize().map_err(|_| LanguageSpecRead)?;
        let path_str = abs.to_str().ok_or(LanguageSpecRead)?;
        match CTX_BUILD_MUTEX.lock() {
            Ok(make_context) => {
                let ctx = make_context(path_str, image.clone()).map_err(|_| SleighInitError)?;
                let mut spaces: Vec<SpaceInfo> = Vec::with_capacity(ctx.getNumSpaces() as usize);
                for idx in 0..ctx.getNumSpaces() {
                    spaces.push(SpaceInfo::from(ctx.getSpaceByIndex(idx)));
                }
                Ok(Self {
                    image,
                    ctx,
                    spaces,
                    language_id: language_def.id.clone(),
                })
            }
            Err(_) => Err(SleighInitError),
        }
    }

    pub(crate) fn set_initial_context(&mut self, name: &str, value: u32) {
        self.ctx.pin_mut().set_initial_context(name, value);
    }

    pub fn read(&self, offset: u64, max_instrs: usize) -> SleighContextInstructionIterator {
        SleighContextInstructionIterator::new(self, offset, max_instrs, false)
    }

    pub fn read_block(&self, offset: u64, max_instrs: usize) -> SleighContextInstructionIterator {
        SleighContextInstructionIterator::new(self, offset, max_instrs, true)
    }

    pub fn spaces(&self) -> Vec<SharedPtr<AddrSpaceHandle>> {
        let mut spaces = Vec::with_capacity(self.ctx.getNumSpaces() as usize);
        for i in 0..self.ctx.getNumSpaces() {
            spaces.push(self.ctx.getSpaceByIndex(i))
        }
        spaces
    }

    pub fn get_language_id(&self) -> &str {
        &self.language_id
    }
}

pub struct SleighContextInstructionIterator<'a> {
    sleigh: &'a SleighContext,
    remaining: usize,
    offset: u64,
    terminate_branch: bool,
    already_hit_branch: bool,
}

impl<'a> SleighContextInstructionIterator<'a> {
    pub(crate) fn new(
        sleigh: &'a SleighContext,
        offset: u64,
        remaining: usize,
        terminate_branch: bool,
    ) -> Self {
        SleighContextInstructionIterator {
            sleigh,
            remaining,
            offset,
            terminate_branch,
            already_hit_branch: false,
        }
    }
}

impl<'a> Iterator for SleighContextInstructionIterator<'a> {
    type Item = Instruction;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        if !self.sleigh.image.contains_address(self.offset as usize) {
            return None;
        }
        if self.terminate_branch && self.already_hit_branch {
            return None;
        }
        let instr = self
            .sleigh
            .ctx
            .get_one_instruction(self.offset)
            .map(Instruction::from)
            .ok()?;
        self.already_hit_branch = instr.terminates_basic_block();
        self.offset += instr.length as u64;
        self.remaining -= 1;
        Some(instr)
    }
}

#[cfg(test)]
mod test {
    use crate::context::builder::image::Image;
    use crate::context::builder::SleighContextBuilder;
    use crate::pcode::PcodeOperation;
    use crate::{Instruction, SpaceManager};

    use crate::tests::SLEIGH_ARCH;
    use crate::varnode;

    #[test]
    fn get_one() {
        let mov_eax_0: [u8; 6] = [0xb8, 0x00, 0x00, 0x00, 0x00, 0xc3];
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let ctx = ctx_builder
            .set_image(Image::from(mov_eax_0.as_slice()))
            .build(SLEIGH_ARCH)
            .unwrap();
        let instr = ctx.read(0, 1).last().unwrap();
        assert_eq!(instr.length, 5);
        assert!(instr.disassembly.mnemonic.eq("MOV"));
        assert!(!instr.ops.is_empty());
        varnode!(&ctx, #0:4).unwrap();
        let _op = PcodeOperation::Copy {
            input: varnode!(&ctx, #0:4).unwrap(),
            output: varnode!(&ctx, "register"[0]:4).unwrap(),
        };
        assert!(matches!(&instr.ops[0], _op))
    }

    #[test]
    fn stop_at_branch() {
        let mov_eax_0: [u8; 4] = [0x0f, 0x05, 0x0f, 0x05];
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let ctx = ctx_builder
            .set_image(Image::from(mov_eax_0.as_slice()))
            .build(SLEIGH_ARCH)
            .unwrap();
        let instr: Vec<Instruction> = ctx.read_block(0, 2).collect();
        assert_eq!(instr.len(), 1);
    }
}
