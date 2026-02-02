pub mod context;
pub(crate) mod error;

pub mod display;
pub(crate) mod ffi;
pub(crate) mod instruction;
pub(crate) mod pcode;
pub(crate) mod space;
pub(crate) mod varnode;

pub use display::{JingleDisplay, JingleDisplayWrapper};
pub use error::JingleSleighError;
pub use ffi::addrspace::bridge::SpaceType;
pub use instruction::*;
pub use pcode::*;
pub use space::{SleighArchInfo, SleighEndianness, SpaceInfo};
pub use varnode::{GeneralizedVarNode, IndirectVarNode, VarNode, create_varnode};

#[cfg(test)]
mod tests {
    pub(crate) const SLEIGH_ARCH: &str = "x86:LE:64:default";
}
