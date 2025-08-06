use crate::error::JingleError;
use jingle_sleigh::{ArchInfoProvider, Instruction, SpaceInfo, VarNode};

use crate::JingleContext;
use crate::modeling::ModeledInstruction;
use jingle_sleigh::JingleSleighError::InstructionDecode;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use z3::Context;

/// This type wraps z3 and a sleigh context and allows for both modeling instructions that
/// sleigh context has already produced, or reading new instructions directly out of sleigh and
/// modeling them in one go
#[derive(Debug, Clone)]
pub struct SleighTranslator<'a> {
    jingle: JingleContext,
    sleigh: &'a LoadedSleighContext<'a>,
}

impl<'a> SleighTranslator<'a> {
    /// Make a new sleigh translator
    pub fn new(sleigh: &'a LoadedSleighContext, z3_ctx: &Context) -> Self {
        let jingle = JingleContext::new(z3_ctx, sleigh);
        Self { jingle, sleigh }
    }

    /// Ask sleigh to read one instruction from the given offset and attempt
    /// to model it
    /// todo: this approach might not work with MIPS delayslots
    pub fn model_instruction_at(&self, offset: u64) -> Result<ModeledInstruction, JingleError> {
        let op = self
            .sleigh
            .instruction_at(offset)
            .ok_or(InstructionDecode)?;
        self.model_instruction(op)
    }

    /// Attempt to convert  the given [Instruction] into a [ModeledInstruction]
    fn model_instruction(&self, instr: Instruction) -> Result<ModeledInstruction, JingleError> {
        ModeledInstruction::new(instr, &self.jingle)
    }
}

impl ArchInfoProvider for SleighTranslator<'_> {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.jingle.get_space_info(idx)
    }

    fn get_all_space_info(&self) -> impl Iterator<Item = &SpaceInfo> {
        self.jingle.get_all_space_info()
    }

    fn get_code_space_idx(&self) -> usize {
        self.jingle.get_code_space_idx()
    }

    fn get_register(&self, name: &str) -> Option<&VarNode> {
        self.jingle.get_register(name)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.jingle.get_register_name(location)
    }

    fn get_registers(&self) -> impl Iterator<Item = (&VarNode, &str)> {
        self.jingle.get_registers()
    }
}
