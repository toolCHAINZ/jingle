use crate::error::JingleError;
use jingle_sleigh::{Instruction, RegisterManager, SpaceInfo, VarNode};

use crate::modeling::ModeledInstruction;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use jingle_sleigh::JingleSleighError::InstructionDecode;
use jingle_sleigh::SpaceManager;
use z3::Context;

/// This type wraps z3 and a sleigh context and allows for both modeling instructions that
/// sleigh context has already produced, or reading new instructions directly out of sleigh and
/// modeling them in one go
#[derive(Debug, Clone)]
pub struct SleighTranslator<'ctx> {
    z3_ctx: &'ctx Context,
    sleigh: &'ctx LoadedSleighContext,
}

impl<'ctx> SleighTranslator<'ctx> {
    /// Make a new sleigh translator
    pub fn new(sleigh: &'ctx LoadedSleighContext, z3_ctx: &'ctx Context) -> Self {
        Self { z3_ctx, sleigh }
    }

    /// Ask sleigh to read one instruction from the given offset and attempt
    /// to model it
    /// todo: this approach might not work with MIPS delayslots
    pub fn model_instruction_at(
        &self,
        offset: u64,
    ) -> Result<ModeledInstruction<'ctx>, JingleError> {
        let op = self
            .sleigh
            .instruction_at(offset)
            .ok_or(InstructionDecode)?;
        self.model_instruction(op)
    }

    /// Attempt to convert  the given [Instruction] into a [ModeledInstruction]
    fn model_instruction(
        &self,
        instr: Instruction,
    ) -> Result<ModeledInstruction<'ctx>, JingleError> {
        ModeledInstruction::new(instr, self.sleigh, self.z3_ctx)
    }
}

impl<'ctx> SpaceManager for SleighTranslator<'ctx> {
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

impl<'ctx> RegisterManager for SleighTranslator<'ctx> {
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
