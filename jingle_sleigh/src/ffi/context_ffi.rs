use crate::ffi::context_ffi::bridge::makeContext;
use bridge::ContextFFI;
use cxx::{Exception, UniquePtr};
use std::sync::Mutex;
use crate::context::image::ImageProvider;

type ContextGeneratorFp = fn(&str) -> Result<UniquePtr<ContextFFI>, Exception>;

pub(crate) static CTX_BUILD_MUTEX: Mutex<ContextGeneratorFp> = Mutex::new(makeContext);

#[cxx::bridge]
pub(crate) mod bridge {
    unsafe extern "C++" {
        type InstructionFFI = crate::ffi::instruction::bridge::InstructionFFI;

        type VarnodeInfoFFI = crate::ffi::instruction::bridge::VarnodeInfoFFI;

        type AddrSpaceHandle = crate::ffi::addrspace::bridge::AddrSpaceHandle;

        type RegisterInfoFFI = crate::ffi::instruction::bridge::RegisterInfoFFI;

    }

    unsafe extern "C++" {
        include!("jingle_sleigh/src/ffi/cpp/context.h");
        include!("jingle_sleigh/src/ffi/cpp/exception.h");

        pub(crate) type ContextFFI;
        pub(super) fn makeContext(slaPath: &str) -> Result<UniquePtr<ContextFFI>>;
        pub(crate) fn set_initial_context(
            self: Pin<&mut ContextFFI>,
            name: &str,
            value: u32,
        ) -> Result<()>;

        pub(crate) fn get_one_instruction(&self, offset: u64) -> Result<InstructionFFI>;

        pub(crate) fn getSpaceByIndex(&self, idx: i32) -> SharedPtr<AddrSpaceHandle>;
        pub(crate) fn getNumSpaces(&self) -> i32;

        // pub(crate) fn getRegister(&self, name: &str) -> Result<VarnodeInfoFFI>;
        // pub(crate) fn getRegisterName(&self, name: VarnodeInfoFFI) -> Result<&str>;

        pub(crate) fn getRegisters(&self) -> Vec<RegisterInfoFFI>;

        pub(crate) fn setImage(self: Pin<&mut ContextFFI>, img: &ImageFFI) -> Result<()>;
    }

    extern "Rust" {
        include!("jingle_sleigh/src/ffi/instruction.rs.h");
        type ImageFFI<'a>;
        fn load(self: &ImageFFI, vn: &VarnodeInfoFFI, out: &mut [u8]) -> usize;
    }
    impl Vec<RegisterInfoFFI> {}
}

pub(crate) struct ImageFFI<'a> {
    pub(crate) provider: Box<dyn ImageProvider + 'a>,
}