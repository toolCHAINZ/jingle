use crate::context::instruction_iterator::SleighContextInstructionIterator;
use crate::context::SleighContext;
use crate::ffi::context_ffi::ImageFFI;
use crate::JingleSleighError::ImageLoadError;
use crate::{Instruction, JingleSleighError, RegisterManager, SpaceInfo, SpaceManager, VarNode};
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use crate::context::image::{ImageProvider, ImageSection};

pub struct LoadedSleighContext<'a> {
    sleigh: SleighContext,
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
    pub(crate) fn new<T: ImageProvider + Sized + 'a>(
        sleigh_context: SleighContext,
        img: T,
    ) -> Result<Self, JingleSleighError> {
        let img = Box::pin(ImageFFI::new(img));
        let mut s = Self {
            sleigh: sleigh_context,
            img
        };
        let (ctx, img) = s.borrow_parts();
        ctx.ctx
            .pin_mut()
            .setImage(img)
            .map_err(|_| ImageLoadError)?;
        Ok(s)
    }
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

    pub fn set_image<T: ImageProvider + Sized + 'a>(&mut self, img: T) -> Result<(), JingleSleighError> {
        let (sleigh, img_ref) = self.borrow_parts();
        *img_ref = ImageFFI::new(img);
        sleigh
            .ctx
            .pin_mut()
            .setImage(img_ref)
            .map_err(|_| ImageLoadError)
    }

    pub fn get_sections<T: ImageProvider + Sized + 'a>(&self) -> impl Iterator<Item=ImageSection> {
        self.img.provider.get_section_info()
    }

    fn borrow_parts<'b>(&'b mut self) -> (&'b mut SleighContext, &'b mut ImageFFI<'a>) {
        (&mut self.sleigh, &mut self.img)
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
