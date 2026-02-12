use crate::Instruction;
use crate::context::loaded::LoadedSleighContext;

pub struct SleighContextInstructionIterator<'a> {
    sleigh: &'a LoadedSleighContext<'a>,
    remaining: usize,
    offset: u64,
    terminate_branch: bool,
    already_hit_branch: bool,
}

impl<'a> SleighContextInstructionIterator<'a> {
    pub(crate) fn new(
        sleigh: &'a LoadedSleighContext,
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

impl Iterator for SleighContextInstructionIterator<'_> {
    type Item = Instruction;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        if self.terminate_branch && self.already_hit_branch {
            return None;
        }
        let mut instr = self
            .sleigh
            .ctx
            .lock()
            .unwrap()
            .get_one_instruction(self.offset)
            .map(Instruction::from)
            .ok()?;
        // Pass the full SleighContext so Instruction::postprocess can consult
        // calling-convention defaults and other context-wide metadata.
        instr.postprocess(self.sleigh);
        self.already_hit_branch = instr.terminates_basic_block();
        self.offset += instr.length as u64;
        self.remaining -= 1;
        Some(instr)
    }
}

#[cfg(test)]
mod test {
    use crate::Instruction;
    use crate::context::builder::SleighContextBuilder;

    use crate::tests::SLEIGH_ARCH;

    #[test]
    fn get_one() {
        let mov_eax_0: [u8; 6] = [0xb8, 0x00, 0x00, 0x00, 0x00, 0xc3];
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let sleigh = sleigh.initialize_with_image(mov_eax_0.as_slice()).unwrap();
        let instr = sleigh.read(0, 1).last().unwrap();
        assert_eq!(instr.length, 5);
        assert!(instr.disassembly.mnemonic.eq("MOV"));
        assert!(!instr.ops.is_empty());
    }

    #[test]
    fn stop_at_branch() {
        let mov_eax_0: Vec<u8> = vec![0x90, 0x90, 0x90, 0x90];
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let sleigh = sleigh.initialize_with_image(mov_eax_0).unwrap();
        let instr: Vec<Instruction> = sleigh.read(0, 5).collect();
        assert_eq!(instr.len(), 4);
    }
}
