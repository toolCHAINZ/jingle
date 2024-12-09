pub mod symbolized;

use crate::ffi::addrspace::bridge::SpaceType;
use crate::{GeneralizedVarNode, IndirectVarNode, VarNode};
use std::fmt::{Display, Formatter, LowerHex, UpperHex};

impl Display for VarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.space._type == SpaceType::IPTR_CONSTANT {
            write!(f, "{}:{}", self.offset, self.size)
        } else {
            write!(f, "{}[{}]:{}", self.space.name, self.offset, self.size)
        }
    }
}

impl LowerHex for VarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.space._type == SpaceType::IPTR_CONSTANT {
            write!(f, "{:x}:{:x}", self.offset, self.size)
        } else {
            write!(f, "{}[{:x}]:{:x}", self.space.name, self.offset, self.size)
        }
    }
}

impl UpperHex for VarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.space._type == SpaceType::IPTR_CONSTANT {
            write!(f, "{:X}:{:X}", self.offset, self.size)
        } else {
            write!(f, "{}[{:X}]:{:X}", self.space.name, self.offset, self.size)
        }
    }
}

impl Display for IndirectVarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "*({}[{}]:{})",
            self.pointer_space.name, self.pointer_location, self.access_size_bytes
        )
    }
}

impl LowerHex for IndirectVarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "*({}[{:x}]:{:x})",
            self.pointer_space.name, self.pointer_location, self.access_size_bytes
        )
    }
}

impl UpperHex for IndirectVarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "*({}[{:X}]:{:X})",
            self.pointer_space.name, self.pointer_location, self.access_size_bytes
        )
    }
}
impl Display for GeneralizedVarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneralizedVarNode::Direct(v) => {
                write!(f, "{v}")
            }
            GeneralizedVarNode::Indirect(v) => {
                write!(f, "{v}")
            }
        }
    }
}

impl LowerHex for GeneralizedVarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneralizedVarNode::Direct(d) => {
                write!(f, "{:x}", d)
            }
            GeneralizedVarNode::Indirect(i) => {
                write!(f, "{:x}", i)
            }
        }
    }
}

impl UpperHex for GeneralizedVarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneralizedVarNode::Direct(d) => {
                write!(f, "{:X}", d)
            }
            GeneralizedVarNode::Indirect(i) => {
                write!(f, "{:X}", i)
            }
        }
    }
}
