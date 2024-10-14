use crate::context::instruction_iterator::SleighContextInstructionIterator;
use crate::context::{Image, SleighContext};
use crate::JingleSleighError::ImageLoadError;
use crate::{Instruction, JingleSleighError, RegisterManager, SpaceInfo, SpaceManager, VarNode};
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

pub struct LoadedSleighContext(SleighContext);

impl Debug for LoadedSleighContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
impl Deref for LoadedSleighContext {
    type Target = SleighContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LoadedSleighContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl LoadedSleighContext {
    pub(crate) fn new(sleigh_context: SleighContext) -> Self {
        Self(sleigh_context)
    }
    pub fn instruction_at(&self, offset: u64) -> Option<Instruction> {
        let instr = self
            .ctx
            .get_one_instruction(offset)
            .map(Instruction::from)
            .ok()?;
        if self
            .image
            .as_ref()?
            .contains_range(offset..(offset + instr.length as u64))
        {
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

    pub fn set_image<T: Into<Image> + Clone>(&mut self, img: T) -> Result<(), JingleSleighError> {
        self.image = Some(img.clone().into());
        self.ctx
            .pin_mut()
            .setImage(img.into())
            .map_err(|_| ImageLoadError)
    }
}

impl SpaceManager for LoadedSleighContext {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.0.get_space_info(idx)
    }

    fn get_all_space_info(&self) -> &[SpaceInfo] {
        self.0.get_all_space_info()
    }

    fn get_code_space_idx(&self) -> usize {
        self.0.get_code_space_idx()
    }
}

impl RegisterManager for LoadedSleighContext {
    fn get_register(&self, name: &str) -> Option<VarNode> {
        self.0.get_register(name)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.0.get_register_name(location)
    }

    fn get_registers(&self) -> Vec<(VarNode, String)> {
        self.0.get_registers()
    }
}
