use crate::context::image::ImageProvider;
use crate::ffi::instruction::bridge::VarnodeInfoFFI;
use crate::VarNode;
use cxx::ExternType;
use crate::ffi::context_ffi::ImageFFI;


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
