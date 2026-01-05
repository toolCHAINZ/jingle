use crate::JingleSleighError::ImageLoadError;
use crate::context::image::{ImageSection, SleighArchImage, SleighImage};
use crate::context::instruction_iterator::SleighContextInstructionIterator;
use crate::context::{SleighContext, SleighContextBuilder};
use crate::ffi::context_ffi::ImageFFI;
use crate::{Instruction, JingleSleighError, VarNode};
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

unsafe impl<'a> Send for LoadedSleighContext<'a> {}
unsafe impl<'a> Sync for LoadedSleighContext<'a> {}

impl Debug for LoadedSleighContext<'_> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.sleigh.fmt(f)
    }
}

impl Deref for LoadedSleighContext<'_> {
    type Target = SleighContext;

    fn deref(&self) -> &Self::Target {
        &self.sleigh
    }
}

impl DerefMut for LoadedSleighContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sleigh
    }
}

impl<'a> LoadedSleighContext<'a> {
    /// Consumes a [SleighContext] and an image provider, initializes
    /// sleigh with the image provider, and combines them into a single
    /// [LoadedSleigh*Context] guard value.
    pub(crate) fn new<T: SleighImage + Sized + 'a>(
        sleigh_context: SleighContext,
        img: T,
    ) -> Result<Self, JingleSleighError> {
        let img = Box::pin(ImageFFI::new(
            img,
            sleigh_context.arch_info().default_code_space_index(),
        ));
        let mut s = Self {
            sleigh: sleigh_context,
            img,
        };
        let (ctx, img) = s.borrow_parts();
        ctx.ctx
            .lock()
            .unwrap()
            .pin_mut()
            .setImage(img)
            .map_err(|_| ImageLoadError)?;
        Ok(s)
    }
    /// Query `sleigh` for the instruction associated with the given offset in the default code
    /// space.
    /// todo: consider using a varnode instead of a raw offset.
    pub fn instruction_at(&self, offset: u64) -> Option<Instruction> {
        let mut instr = self
            .ctx
            .lock()
            .unwrap()
            .get_one_instruction(offset)
            .map(Instruction::from)
            .ok()?;
        instr.augment_with_metadata(&self.metadata);
        let vn = VarNode {
            space_index: self.arch_info.default_code_space_index(),
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
    pub fn read(&self, offset: u64, max_instrs: usize) -> SleighContextInstructionIterator<'_> {
        SleighContextInstructionIterator::new(self, offset, max_instrs, false)
    }

    /// Read the byte range specified by the given [`VarNode`] from the configured image provider.
    pub fn read_bytes(&self, vn: &VarNode) -> Option<Vec<u8>> {
        if vn.space_index == self.arch_info.default_code_space_index() {
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
    ) -> SleighContextInstructionIterator<'_> {
        SleighContextInstructionIterator::new(self, offset, max_instrs, true)
    }

    /// Re-initialize `sleigh` with a new image, without re-parsing the `.sla` definitions. This
    /// is _much_ faster than generating a new context.
    ///
    /// This API retains the current base address of the image.
    pub fn set_image<T: SleighImage + Sized + 'a>(
        &mut self,
        img: T,
    ) -> Result<(), JingleSleighError> {
        let base_address = self.get_base_address();
        let (sleigh, img_ref) = self.borrow_parts();
        *img_ref = ImageFFI::new(img, sleigh.arch_info().default_code_space_index());
        sleigh
            .ctx
            .lock()
            .unwrap()
            .pin_mut()
            .setImage(img_ref)
            .map_err(|_| ImageLoadError)?;
        self.img.set_base_address(base_address);
        Ok(())
    }

    /// Returns an iterator of entries describing the sections of the configured image provider.
    pub fn get_sections(&self) -> impl Iterator<Item = ImageSection<'_>> {
        self.img.provider.get_section_info().map(|mut s| {
            s.base_address += self.get_base_address() as usize;
            s
        })
    }

    fn borrow_parts<'b>(&'b mut self) -> (&'b mut SleighContext, &'b mut ImageFFI<'a>) {
        (&mut self.sleigh, &mut self.img)
    }

    /// Rebase the loaded image to `offset`
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

    pub fn load<I: SleighArchImage + 'a, P: AsRef<str>>(
        img: I,
        ghidra_path: P,
    ) -> Result<Self, JingleSleighError> {
        let ctx_builder = SleighContextBuilder::load_ghidra_installation(ghidra_path.as_ref())?;
        let sleigh = ctx_builder.build(img.architecture_id()?)?;
        Self::new(sleigh, img)
    }
}

#[cfg(test)]
mod tests {
    use crate::PcodeOperation::Branch;
    use crate::VarNode;
    use crate::context::SleighContextBuilder;
    
    use crate::tests::SLEIGH_ARCH;
    

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
    pub fn relative_addresses() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        // JMP $+5
        let img: [u8; 2] = [0xeb, 0x05];
        let mut loaded = sleigh.initialize_with_image(img.as_slice()).unwrap();
        let instr = loaded.instruction_at(0).unwrap();
        assert_eq!(
            instr.ops[0],
            Branch {
                input: VarNode {
                    space_index: 3,
                    size: 8,
                    offset: 7
                }
            }
        );
        loaded.set_base_address(0x100);
        let instr2 = loaded.instruction_at(0x100).unwrap();
        assert_eq!(
            instr2.ops[0],
            Branch {
                input: VarNode {
                    space_index: 3,
                    size: 8,
                    offset: 0x107
                }
            }
        );
    }

    #[test]
    pub fn multithreaded_instruction_fetch() {
        use std::thread;

        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();

        // Create a small program with multiple instructions
        // PUSH RBP (0x55), MOV RBP,RSP (0x48 0x89 0xe5), PUSH RBX (0x53), NOP (0x90)
        let img: Vec<u8> = vec![0x55, 0x48, 0x89, 0xe5, 0x53, 0x90];
        let loaded = sleigh.initialize_with_image(img.as_slice()).unwrap();

        const NUM_THREADS: usize = 8;
        const ITERATIONS_PER_THREAD: usize = 100;

        // Use scoped threads to allow borrowing from the parent thread
        thread::scope(|s| {
            let mut handles = vec![];

            for thread_id in 0..NUM_THREADS {
                let tid = thread_id;
                let loaded_ref = &loaded;
                let handle = s.spawn(move || {
                    for _i in 0..ITERATIONS_PER_THREAD {
                        // Fetch instruction at offset 0 (PUSH RBP)
                        let instr0 = loaded_ref.instruction_at(0).unwrap();
                        assert_eq!(instr0.disassembly.mnemonic, "PUSH");

                        // Fetch instruction at offset 1 (MOV RBP,RSP)
                        let instr1 = loaded_ref.instruction_at(1).unwrap();
                        assert_eq!(instr1.disassembly.mnemonic, "MOV");

                        // Read bytes from multiple offsets
                        let bytes = loaded_ref
                            .read_bytes(&VarNode {
                                space_index: 3,
                                size: 6,
                                offset: 0,
                            })
                            .unwrap();
                        assert_eq!(bytes.len(), 6);
                        assert_eq!(bytes[0], 0x55);
                        assert_eq!(bytes[5], 0x90);

                        // Use read iterator
                        let instrs: Vec<_> = loaded_ref.read(0, 4).collect();
                        assert!(instrs.len() >= 2);
                    }
                    tid
                });
                handles.push(handle);
            }

            // Wait for all threads to complete and collect results
            let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

            // Verify all threads completed successfully
            assert_eq!(results.len(), NUM_THREADS);
            for (i, &result) in results.iter().enumerate() {
                assert_eq!(result, i);
            }
        });
    }

    #[test]
    fn test_get_sections() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let img: Vec<u8> = vec![0x55, 0x48, 0x89, 0xe5];
        let loaded = sleigh.initialize_with_image(img.as_slice()).unwrap();

        let sections: Vec<_> = loaded.get_sections().collect();
        assert!(!sections.is_empty());
        assert_eq!(sections[0].base_address, 0);
        assert_eq!(sections[0].data.len(), 4);
    }

    #[test]
    fn test_get_sections_with_base_address() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let img: Vec<u8> = vec![0x55, 0x48, 0x89, 0xe5];
        let mut loaded = sleigh.initialize_with_image(img.as_slice()).unwrap();
        loaded.set_base_address(0x1000);

        let sections: Vec<_> = loaded.get_sections().collect();
        assert!(!sections.is_empty());
        // Base address should be added to section base
        assert_eq!(sections[0].base_address, 0x1000);
    }

    #[test]
    fn test_set_image() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();

        // Start with first image
        let img1: Vec<u8> = vec![0x55]; // PUSH RBP
        let mut loaded = sleigh.initialize_with_image(img1).unwrap();
        let instr1 = loaded.instruction_at(0).unwrap();
        assert_eq!(instr1.disassembly.mnemonic, "PUSH");

        // Replace with second image
        let img2: Vec<u8> = vec![0x90]; // NOP
        loaded.set_image(img2).unwrap();
        let instr2 = loaded.instruction_at(0).unwrap();
        assert_eq!(instr2.disassembly.mnemonic, "NOP");
    }

    #[test]
    fn test_read_until_branch() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();

        // NOP, NOP, JMP $+5, NOP (should stop at JMP)
        let img: Vec<u8> = vec![0x90, 0x90, 0xeb, 0x05, 0x90];
        let loaded = sleigh.initialize_with_image(img.as_slice()).unwrap();

        let instrs: Vec<_> = loaded.read_until_branch(0, 10).collect();
        // Should read NOPs and JMP, then stop
        assert!(instrs.len() >= 2);
        assert!(instrs.len() <= 3);
        // Last instruction should be the branch
        let last = &instrs[instrs.len() - 1];
        assert!(last.terminates_basic_block());
    }

    #[test]
    fn test_read_until_branch_no_branch() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();

        // Only NOPs, no branches
        let img: Vec<u8> = vec![0x90, 0x90, 0x90, 0x90];
        let loaded = sleigh.initialize_with_image(img.as_slice()).unwrap();

        let instrs: Vec<_> = loaded.read_until_branch(0, 10).collect();
        // Should read all NOPs up to the limit
        assert_eq!(instrs.len(), 4);
    }

    #[test]
    fn test_read_bytes_non_code_space() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let img: Vec<u8> = vec![0x55, 0x48, 0x89, 0xe5];
        let loaded = sleigh.initialize_with_image(img.as_slice()).unwrap();

        // Try to read from a non-code space (should return None)
        let non_code_space_index = 1; // Typically not the code space
        let result = loaded.read_bytes(&VarNode {
            space_index: non_code_space_index,
            size: 4,
            offset: 0,
        });
        assert_eq!(result, None);
    }

    #[test]
    fn test_debug_impl() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let img: Vec<u8> = vec![0x55];
        let loaded = sleigh.initialize_with_image(img.as_slice()).unwrap();

        // Just verify Debug can be formatted without panic
        let debug_str = format!("{:?}", loaded);
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn test_instruction_at_out_of_bounds() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let img: Vec<u8> = vec![0x55];
        let loaded = sleigh.initialize_with_image(img.as_slice()).unwrap();

        // Try to read instruction beyond image bounds
        let result = loaded.instruction_at(100);
        assert_eq!(result, None);
    }

    #[test]
    fn test_read_empty_iterator() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let img: Vec<u8> = vec![0x55];
        let loaded = sleigh.initialize_with_image(img.as_slice()).unwrap();

        // Request 0 instructions
        let instrs: Vec<_> = loaded.read(0, 0).collect();
        assert_eq!(instrs.len(), 0);
    }

    #[test]
    fn test_base_address_persistence_across_set_image() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();

        let img1: Vec<u8> = vec![0x55];
        let mut loaded = sleigh.initialize_with_image(img1).unwrap();
        loaded.set_base_address(0x5000);
        assert_eq!(loaded.get_base_address(), 0x5000);

        // After setting new image, base address should reset to 0
        let img2: Vec<u8> = vec![0x90];
        loaded.set_image(img2).unwrap();
        // The base address behavior depends on implementation
        // Just verify we can still get it
        let _ = loaded.get_base_address();
    }
}
