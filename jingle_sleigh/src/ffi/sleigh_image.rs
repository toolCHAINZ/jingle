#[cxx::bridge]
pub(crate) mod bridge {
    unsafe extern "C++" {
        type Image = crate::context::Image;
        type InstructionFFI = crate::ffi::instruction::bridge::InstructionFFI;

        type VarnodeInfoFFI = crate::ffi::instruction::bridge::VarnodeInfoFFI;

        type AddrSpaceHandle = crate::ffi::addrspace::bridge::AddrSpaceHandle;

        type RegisterInfoFFI = crate::ffi::instruction::bridge::RegisterInfoFFI;
    }

    unsafe extern "C++" {
        include!("jingle_sleigh/src/ffi/cpp/sleigh_image.h");

        pub(crate) type SleighImage;

        pub(crate) fn getSpaceByIndex(&self, idx: i32) -> SharedPtr<AddrSpaceHandle>;
        pub(crate) fn getNumSpaces(&self) -> i32;

        pub(crate) fn getRegister(&self, name: &str) -> Result<VarnodeInfoFFI>;
        pub(crate) fn getRegisterName(&self, name: VarnodeInfoFFI) -> Result<&str>;

        pub(crate) fn getRegisters(&self) -> Vec<RegisterInfoFFI>;

        pub(crate) fn get_one_instruction(&self, offset: u64) -> Result<InstructionFFI>;
    }

    impl UniquePtr<SleighImage> {}
}
