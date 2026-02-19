use crate::error::JingleError;
use jingle_sleigh::{Instruction, SleighArchInfo};

use crate::modeling::ModeledInstruction;
use jingle_sleigh::JingleSleighError::InstructionDecode;
use jingle_sleigh::context::image::SleighImage;
use jingle_sleigh::context::loaded::LoadedSleighContext;

/// This type wraps z3 and a sleigh context and allows for both modeling instructions that
/// sleigh context has already produced, or reading new instructions directly out of sleigh and
/// modeling them in one go
#[derive(Debug, Clone)]
pub struct SleighTranslator<'a, T: SleighImage> {
    info: SleighArchInfo,
    sleigh: &'a LoadedSleighContext<'a, T>,
}

impl<'a, T: SleighImage> SleighTranslator<'a, T> {
    /// Make a new sleigh translator
    pub fn new(sleigh: &'a LoadedSleighContext<'a, T>) -> Self {
        Self {
            info: sleigh.arch_info().clone(),
            sleigh,
        }
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
        ModeledInstruction::new(instr, &self.info)
    }
}
