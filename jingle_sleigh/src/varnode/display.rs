use crate::ffi::addrspace::bridge::SpaceType;
use crate::space::SpaceInfo;
use crate::{GeneralizedVarNode, IndirectVarNode, VarNode};
#[cfg(feature = "pyo3")]
use pyo3::{pyclass};
use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "pyo3", pyclass(str))]
pub enum VarNodeDisplay {
    Raw(RawVarNodeDisplay),
    Register(String, VarNode),
}
#[derive(Clone, Debug)]
#[cfg_attr(feature = "pyo3", pyclass(str))]
pub struct RawVarNodeDisplay {
    pub offset: u64,
    pub size: usize,
    pub space_info: SpaceInfo,
}

#[derive(Clone, Debug)]
pub struct IndirectVarNodeDisplay {
    pub pointer_space_info: SpaceInfo,
    pub pointer_location: VarNodeDisplay,
    pub access_size_bytes: usize,
}

#[derive(Clone, Debug)]
pub enum GeneralizedVarNodeDisplay {
    Direct(VarNodeDisplay),
    Indirect(IndirectVarNodeDisplay),
}

impl Display for RawVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.space_info._type == SpaceType::IPTR_CONSTANT {
            write!(f, "{:x}:{:x}", self.offset, self.size)
        } else {
            write!(
                f,
                "{}[{:x}]:{:x}",
                self.space_info.name, self.offset, self.size
            )
        }
    }
}
impl Display for VarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VarNodeDisplay::Raw(r) => {
                write!(f, "{}", r)
            }
            VarNodeDisplay::Register(a, ..) => {
                write!(f, "{}", a)
            }
        }
    }
}

impl Display for IndirectVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "*({}[{}]:{})",
            self.pointer_space_info.name, self.pointer_location, self.access_size_bytes
        )
    }
}

impl Display for GeneralizedVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneralizedVarNodeDisplay::Direct(v) => {
                write!(f, "{v}")
            }
            GeneralizedVarNodeDisplay::Indirect(v) => {
                write!(f, "{v}")
            }
        }
    }
}

impl From<RawVarNodeDisplay> for VarNode {
    fn from(v: RawVarNodeDisplay) -> Self {
        VarNode {
            space_index: v.space_info.index,
            offset: v.offset,
            size: v.size,
        }
    }
}

impl From<VarNodeDisplay> for VarNode {
    fn from(value: VarNodeDisplay) -> Self {
        match value {
            VarNodeDisplay::Raw(v) => v.into(),
            VarNodeDisplay::Register(_, a) => a,
        }
    }
}

impl From<IndirectVarNodeDisplay> for IndirectVarNode {
    fn from(value: IndirectVarNodeDisplay) -> Self {
        IndirectVarNode {
            pointer_space_index: value.pointer_space_info.index,
            pointer_location: value.pointer_location.into(),
            access_size_bytes: value.access_size_bytes,
        }
    }
}

impl From<GeneralizedVarNodeDisplay> for GeneralizedVarNode {
    fn from(value: GeneralizedVarNodeDisplay) -> Self {
        match value {
            GeneralizedVarNodeDisplay::Direct(d) => GeneralizedVarNode::Direct(d.into()),
            GeneralizedVarNodeDisplay::Indirect(i) => GeneralizedVarNode::Indirect(i.into()),
        }
    }
}
