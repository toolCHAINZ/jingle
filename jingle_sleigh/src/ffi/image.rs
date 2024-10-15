use std::fmt::{Debug, Formatter};
use crate::context::image::ImageProvider;
use crate::ffi::instruction::bridge::VarnodeInfoFFI;
use crate::VarNode;

pub(crate) struct ImageFFI{
    provider: Box<dyn ImageProvider>
}

impl ImageFFI{
    pub(crate) fn new<T: ImageProvider>(provider: T) -> Self{
        Self{provider: Box::new(provider)}
    }
    fn load(&self, vn: &VarnodeInfoFFI, out: &mut [u8]) -> usize{
        self.provider.load(VarNode::from(vn), out)
    }
}

#[cxx::bridge]
pub(crate) mod bridge {
    extern "C++"{
        type VarnodeInfoFFI = crate::ffi::instruction::bridge::VarnodeInfoFFI;
    }
    extern "Rust"{

        type ImageFFI;
        fn load(self: &ImageFFI, vn: &VarnodeInfoFFI, out: &mut [u8]) -> usize;
    }
}
