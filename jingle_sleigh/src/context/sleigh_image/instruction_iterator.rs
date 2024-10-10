use crate::context::sleigh_image::SleighImage;
use crate::context::{Image, SleighContext};
use crate::Instruction;

pub struct SleighContextInstructionIterator<'a> {
    sleigh: &'a SleighImage,
    remaining: usize,
    offset: u64,
    terminate_branch: bool,
    already_hit_branch: bool,
}

impl<'a> SleighContextInstructionIterator<'a> {
    pub(crate) fn new(
        sleigh: &'a SleighImage,
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
        if self.terminate_branch && self.already_hit_branch {
            return None;
        }
        let instr = self
            .sleigh
            .img_ffi
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
    use crate::{Instruction, RegisterManager, SpaceManager};

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
