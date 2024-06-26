pub(crate) mod addrspace;
#[cfg(compile)]
pub(crate) mod compile;
pub(crate) mod context_ffi;
pub(crate) mod image;
pub(crate) mod instruction;
pub(crate) mod opcode;

#[cfg(test)]
mod tests {
    use crate::context::{Image, SleighContextBuilder};

    #[test]
    fn test_callother_decode() {
        let bytes: Vec<u8> = vec![0xf0, 0x0f, 0xb1, 0x95, 0xac, 0x15, 0x00, 0x00];
        let builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();

        let bin_sleigh = builder
            .set_image(Image::try_from(bytes.as_slice()).unwrap())
            .build("x86:LE:64:default")
            .unwrap();
        let _lib = bin_sleigh.read(0, 1).next().unwrap();
    }
    #[test]
    fn test_callother_decode2() {
        let bytes: Vec<u8> = vec![0xf0, 0x0f, 0xb1, 0x95, 0xac, 0x15, 0x00, 0x00];
        let builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();

        let bin_sleigh = builder
            .set_image(Image::try_from(bytes.as_slice()).unwrap())
            .build("x86:LE:64:default")
            .unwrap();
        let _lib = bin_sleigh.read(0, 1).next().unwrap();
    }
}
