use crate::context::image::{ImageProvider, ImageSection};
use crate::context::instruction_iterator::SleighContextInstructionIterator;
use crate::context::SleighContext;
use crate::ffi::context_ffi::ImageFFI;
use crate::JingleSleighError::ImageLoadError;
use crate::{Instruction, JingleSleighError, RegisterManager, SpaceInfo, SpaceManager, VarNode};
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;

/// A guard type representing a sleigh context initialized with an image.
/// In addition to the methods in [SleighContext], is able to
/// query bytes for address ranges from its source image, as well
/// as ISA instructions (and associated `p-code`).
pub struct LoadedSleighContext<'a> {
    /// A handle to `sleigh`. By construction, this context is initialized with an image
    sleigh: SleighContext,
    /// A handle to the image source being queried by the [SleighContext].
    img: Pin<Box<ImageFFI<'a>>>,
}

impl<'a> Debug for LoadedSleighContext<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.sleigh.fmt(f)
    }
}
impl<'a> Deref for LoadedSleighContext<'a> {
    type Target = SleighContext;

    fn deref(&self) -> &Self::Target {
        &self.sleigh
    }
}

impl<'a> DerefMut for LoadedSleighContext<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sleigh
    }
}

impl<'a> LoadedSleighContext<'a> {
    /// Consumes a [SleighContext] and an image provider, initializes
    /// sleigh with the image provider, and combines them into a single
    /// [LoadedSleigh*Context] guard value.
    pub(crate) fn new<T: ImageProvider + Sized + 'a>(
        sleigh_context: SleighContext,
        img: T,
    ) -> Result<Self, JingleSleighError> {
        let img = Box::pin(ImageFFI::new(img));
        let mut s = Self {
            sleigh: sleigh_context,
            img,
        };
        let (ctx, img) = s.borrow_parts();
        ctx.ctx
            .pin_mut()
            .setImage(img)
            .map_err(|_| ImageLoadError)?;
        Ok(s)
    }
    /// Query `sleigh` for the instruction associated with the given offset in the default code
    /// space.
    /// todo: consider using a varnode instead of a raw offset.
    pub fn instruction_at(&self, offset: u64) -> Option<Instruction> {
        let instr = self
            .ctx
            .get_one_instruction(offset)
            .map(Instruction::from)
            .ok()?;
        let vn = VarNode {
            space_index: self.sleigh.get_code_space_idx(),
            size: instr.length,
            offset,
        };
        if self.img.has_range(&vn) {
            Some(instr)
        } else {
            None
        }
    }

    /// Read an iterator of at most `max_instrs` [`Instruction`]s from `offset` in the default code
    /// space.
    /// todo: consider using a varnode instead of a raw offset
    pub fn read(&self, offset: u64, max_instrs: usize) -> SleighContextInstructionIterator {
        SleighContextInstructionIterator::new(
            self,
            offset,
            max_instrs,
            false,
        )
    }

    /// Read the byte range specified by the given [`VarNode`] from the configured image provider.
    pub fn read_bytes(&self, vn: &VarNode) -> Option<Vec<u8>> {
        if vn.space_index == self.get_code_space_idx() {
            self.img.provider.get_bytes(&self.adjust_varnode_vma(vn))
        } else {
            None
        }
    }

    /// Read an iterator of at most `max_instrs` [`Instruction`]s from `offset` in the default code
    /// space, terminating if a branch is encountered.
    /// todo: consider using a varnode instead of a raw offset
    pub fn read_until_branch(
        &self,
        offset: u64,
        max_instrs: usize,
    ) -> SleighContextInstructionIterator {
        SleighContextInstructionIterator::new(
            self,
            offset,
            max_instrs,
            true,
        )
    }

    /// Re-initialize `sleigh` with a new image, without re-parsing the `.sla` definitions. This
    /// is _much_ faster than generating a new context.
    pub fn set_image<T: ImageProvider + Sized + 'a>(
        &mut self,
        img: T,
    ) -> Result<(), JingleSleighError> {
        let (sleigh, img_ref) = self.borrow_parts();
        *img_ref = ImageFFI::new(img);
        sleigh
            .ctx
            .pin_mut()
            .setImage(img_ref)
            .map_err(|_| ImageLoadError)
    }

    /// Returns an iterator of entries describing the sections of the configured image provider.
    pub fn get_sections(&self) -> impl Iterator<Item = ImageSection> {
        self.img.provider.get_section_info().map(|mut s| {
            s.base_address += self.get_base_address() as usize;
            s
        })
    }

    fn borrow_parts<'b>(&'b mut self) -> (&'b mut SleighContext, &'b mut ImageFFI<'a>) {
        (&mut self.sleigh, &mut self.img)
    }

    /// Rebase the loaded image to `offset`
    pub fn set_base_address(&mut self, offset: u64) {
        self.img.set_base_address(offset);
    }

    /// Get the current base address
    pub fn get_base_address(&self) -> u64 {
        self.img.get_base_address()
    }

    // todo: properly account for spaces with non-byte-based indexing
    fn adjust_varnode_vma(&self, vn: &VarNode) -> VarNode {
        VarNode {
            space_index: vn.space_index,
            size: vn.size,
            offset: vn.offset.wrapping_sub(self.get_base_address()),
        }
    }
}

impl<'a> SpaceManager for LoadedSleighContext<'a> {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.sleigh.get_space_info(idx)
    }

    fn get_all_space_info(&self) -> &[SpaceInfo] {
        self.sleigh.get_all_space_info()
    }

    fn get_code_space_idx(&self) -> usize {
        self.sleigh.get_code_space_idx()
    }
}

impl<'a> RegisterManager for LoadedSleighContext<'a> {
    fn get_register(&self, name: &str) -> Option<VarNode> {
        self.sleigh.get_register(name)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.sleigh.get_register_name(location)
    }

    fn get_registers(&self) -> Vec<(VarNode, String)> {
        self.sleigh.get_registers()
    }
}

#[cfg(test)]
mod tests {
    use crate::context::SleighContextBuilder;
    use crate::PcodeOperation::Branch;
    use crate::tests::SLEIGH_ARCH;
    use crate::VarNode;

    #[test]
    fn test_adjust_vma() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let img: [u8; 5] = [0x55, 1, 2, 3, 4];
        let mut loaded = sleigh.initialize_with_image(img.as_slice()).unwrap();
        let first = loaded
            .read_bytes(&VarNode {
                space_index: 3,
                size: 5,
                offset: 0,
            })
            .unwrap();
        assert_eq!(first.as_slice(), img.as_slice());
        let instr1 = loaded.instruction_at(0).unwrap();
        assert_eq!(instr1.disassembly.mnemonic, "PUSH");
        loaded.set_base_address(100);
        assert!(loaded.instruction_at(0).is_none());
        assert_eq!(
            loaded.read_bytes(&VarNode {
                space_index: 3,
                size: 5,
                offset: 0
            }),
            None
        );
        let second = loaded
            .read_bytes(&VarNode {
                space_index: 3,
                size: 5,
                offset: 100,
            })
            .unwrap();
        assert_eq!(second.as_slice(), img.as_slice());
        let instr2 = loaded.instruction_at(100).unwrap();
        assert_eq!(instr2.disassembly.mnemonic, "PUSH");
        for (a, b) in instr2.ops.iter().zip(instr1.ops) {
            assert_eq!(a.opcode(), b.opcode())
        }
    }

    #[test]
    pub fn relative_addresses(){
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        // JMP $+5
        let img: [u8; 2] = [0xeb, 0x05];
        let mut loaded = sleigh.initialize_with_image(img.as_slice()).unwrap();
        let instr = loaded.instruction_at(0).unwrap();
        assert_eq!(instr.ops[0], Branch {input: VarNode{ space_index: 3, size: 8, offset: 7}});
        loaded.set_base_address(0x100);
        let instr2 = loaded.instruction_at(0x100).unwrap();
        assert_eq!(instr2.ops[0], Branch {input: VarNode{ space_index: 3, size: 8, offset: 0x107}});

    }
}
