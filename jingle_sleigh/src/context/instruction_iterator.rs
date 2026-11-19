use crate::Instruction;
use crate::context::ContextState;
use crate::context::loaded::LoadedSleighContext;

pub struct SleighContextInstructionIterator<'a> {
    sleigh: &'a LoadedSleighContext<'a>,
    remaining: usize,
    offset: u64,
    terminate_branch: bool,
    already_hit_branch: bool,
    /// Identifies this iteration as the owner of the context database's current state.
    token: u64,
    /// Offsets decoded so far. If another caller decodes through the context between two calls
    /// to `next`, these are re-decoded to restore the context this iteration has committed.
    history: Vec<u64>,
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
            token: ContextState::new_token(),
            history: Vec::new(),
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
        let raw = {
            let mut state = self.sleigh.ctx.lock().unwrap();
            if state.owner != self.token {
                // First call, or someone else has decoded since our last call: start from default
                // context and replay our own decodes to recommit their context changes.
                state.pin_mut().reset_context(false).ok()?;
                for &offset in &self.history {
                    state.get_one_instruction(offset).ok()?;
                }
                state.owner = self.token;
            }
            state.get_one_instruction(self.offset).ok()?
        };
        self.history.push(self.offset);
        let instr = self.sleigh.finish_instruction(raw)?;
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
            SleighContextBuilder::load_ghidra_installation(crate::tests::ghidra_path()).unwrap();
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
            SleighContextBuilder::load_ghidra_installation(crate::tests::ghidra_path()).unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let sleigh = sleigh.initialize_with_image(mov_eax_0).unwrap();
        let instr: Vec<Instruction> = sleigh.read(0, 5).collect();
        assert_eq!(instr.len(), 4);
    }
}
