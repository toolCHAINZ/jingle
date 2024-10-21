use crate::ffi::addrspace::bridge::SpaceType;
use crate::space::SpaceInfo;
use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Debug)]
pub enum VarNodeDisplay{
    Raw(RawVarNodeDisplay),
    Register(String)
}
#[derive(Clone, Debug)]
pub struct RawVarNodeDisplay {
    pub offset: u64,
    pub size: usize,
    pub space_info: SpaceInfo,
}

#[derive(Clone, Debug)]
pub struct IndirectVarNodeDisplay {
    pub pointer_space_name: String,
    pub pointer_location: VarNodeDisplay,
    pub access_size_bytes: usize,
}

#[derive(Clone, Debug)]
pub enum GeneralizedVarNodeDisplay {
    Direct(VarNodeDisplay),
    Indirect(IndirectVarNodeDisplay),
}

impl Display for RawVarNodeDisplay{
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
        match self{
            VarNodeDisplay::Raw(r) => {write!(f, "{}", r)}
            VarNodeDisplay::Register(a) => {write!(f, "{}", a)}
        }

    }
}

impl Display for IndirectVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "*({}[{}]:{})",
            self.pointer_space_name, self.pointer_location, self.access_size_bytes
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
