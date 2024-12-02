use crate::context::image::ImageProvider;
use crate::ffi::context_ffi::bridge::makeContext;
use crate::ffi::instruction::bridge::VarnodeInfoFFI;
use crate::{SpaceInfo, VarNode};
use bridge::ContextFFI;
use cxx::{Exception, ExternType, UniquePtr};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Mutex;
use crate::space::SharedSpaceInfo;

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
    /// A thing that has bytes at addresses
    pub(crate) provider: Pin<Box<dyn ImageProvider + 'a>>,
    /// The current virtual base address for the image loaded by this context.
    pub(crate) base_offset: u64,
    /// The space that this image is attached to. For now, always the
    /// default code space.
    pub(crate) space: SharedSpaceInfo,
}

impl<'a> ImageFFI<'a> {
    pub(crate) fn new<T: ImageProvider + 'a>(provider: T, space: Rc<SpaceInfo>) -> Self {
        Self {
            provider: Box::pin(provider),
            base_offset: 0,
            space: space.into(),
        }
    }
    pub(crate) fn load(&self, vn: &VarnodeInfoFFI, out: &mut [u8]) -> usize {
        if vn.space.getIndex() as usize != self.space.index {
            return 0;
        }
        let addr = VarNode {
            space: self.space.clone(),
            offset: vn.offset,
            size: vn.size,
        };

        let adjusted = self.adjust_varnode_vma(&addr);
        self.provider.load(&adjusted, out)
    }

    pub(crate) fn has_range(&self, vn: &VarNode) -> bool {
        if vn.space.index != self.space.index {
            return false;
        }
        self.provider.has_full_range(&self.adjust_varnode_vma(vn))
    }

    pub(crate) fn get_base_address(&self) -> u64 {
        self.base_offset
    }

    pub(crate) fn set_base_address(&mut self, offset: u64) {
        self.base_offset = offset
    }
    // todo: properly account for spaces with non-byte-based indexing
    fn adjust_varnode_vma(&self, vn: &VarNode) -> VarNode {
        VarNode {
            space: vn.space.clone(),
            size: vn.size,
            offset: vn.offset.wrapping_sub(self.base_offset),
        }
    }
}

unsafe impl<'a> ExternType for ImageFFI<'a> {
    type Id = cxx::type_id!("ImageFFI");
    type Kind = cxx::kind::Opaque;
}
