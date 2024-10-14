pub(crate) mod addrspace;
pub(crate) mod context_ffi;
pub(crate) mod image;
pub(crate) mod instruction;
pub(crate) mod opcode;

// Need to pull this in somewhere so that libz symbols are available
// for the `sleigh` CPP code at link-time.
#[allow(unused_imports)]
use libz_sys::inflate;

#[cfg(test)]
mod tests {
    use crate::context::SleighContextBuilder;
    use crate::tests::SLEIGH_ARCH;

    #[test]
    fn test_callother_decode() {
        let bytes: Vec<u8> = vec![0xf0, 0x0f, 0xb1, 0x95, 0xac, 0x15, 0x00, 0x00];
        let builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();

        let mut sleigh = builder.build("x86:LE:64:default").unwrap();
        let sleigh = sleigh.set_image(bytes.as_slice()).unwrap();
        sleigh.instruction_at(0).unwrap();
    }
    #[test]
    fn test_callother_decode2() {
        let bytes: Vec<u8> = vec![0xf0, 0x0f, 0xb1, 0x95, 0xac, 0x15, 0x00, 0x00];
        let builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();

        let mut sleigh = builder.build("x86:LE:64:default").unwrap();
        let sleigh = sleigh.set_image(bytes.as_slice()).unwrap();
        sleigh.instruction_at(0).unwrap();
    }

    #[test]
    fn test_two_images() {
        let mov_eax_0: [u8; 4] = [0x0f, 0x05, 0x0f, 0x05];
        let nops: [u8; 9] = [0x90, 0x90, 0x90, 0x90, 0x0f, 0x05, 0x0f, 0x05, 0x0f];
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let mut sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let mut sleigh = sleigh.set_image(mov_eax_0.as_slice()).unwrap();
        let instr1 = sleigh.instruction_at(0);
        sleigh.set_image(nops.as_slice()).unwrap();
        let instr2 = sleigh.instruction_at(0);
        assert_ne!(instr1, instr2);
        assert_ne!(instr1, None);
        let instr2 = sleigh.instruction_at(4);
        assert_ne!(instr1, instr2);
        assert_ne!(instr2, None);
        let instr3 = sleigh.instruction_at(8);
        assert_eq!(instr3, None);
    }
}
