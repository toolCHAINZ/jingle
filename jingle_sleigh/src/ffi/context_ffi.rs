use crate::context::image::ImageProvider;
use crate::ffi::context_ffi::bridge::makeContext;
use crate::ffi::instruction::bridge::VarnodeInfoFFI;
use crate::VarNode;
use bridge::ContextFFI;
use cxx::{Exception, ExternType, UniquePtr};
use std::pin::Pin;
use std::sync::Mutex;

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
    pub(crate) provider: Pin<Box<dyn ImageProvider + 'a>>,
}

impl<'a> ImageFFI<'a> {
    pub(crate) fn new<T: ImageProvider + 'a>(provider: T) -> Self {
        Self {
            provider: Box::pin(provider),
        }
    }
    pub(crate) fn load(&self, vn: &VarnodeInfoFFI, out: &mut [u8]) -> usize {
        self.provider.load(&VarNode::from(vn), out)
    }

    pub(crate) fn has_range(&self, vn: &VarNode) -> bool {
        self.provider.has_full_range(vn)
    }
}

unsafe impl<'a> ExternType for ImageFFI<'a> {
    type Id = cxx::type_id!("ImageFFI");
    type Kind = cxx::kind::Opaque;
}
