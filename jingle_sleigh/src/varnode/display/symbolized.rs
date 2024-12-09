use crate::{SharedSpaceInfo, VarNode};
use std::fmt::{Display, Formatter, LowerHex, UpperHex};

/// A helper struct for representing VarNodes that might be better represented as a symbol
/// (e.g. a register or a branch target)
pub enum SymbolizedVarNodeDisplay {
    Symbol(String),
    VarNode(VarNode),
}

impl Display for SymbolizedVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolizedVarNodeDisplay::Symbol(s) => write!(f, "{}", s),
            SymbolizedVarNodeDisplay::VarNode(s) => write!(f, "{}", s),
        }
    }
}

impl LowerHex for SymbolizedVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolizedVarNodeDisplay::Symbol(s) => write!(f, "{}", s),
            SymbolizedVarNodeDisplay::VarNode(s) => write!(f, "{:x}", s),
        }
    }
}

impl UpperHex for SymbolizedVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolizedVarNodeDisplay::Symbol(s) => write!(f, "{}", s),
            SymbolizedVarNodeDisplay::VarNode(s) => write!(f, "{:X}", s),
        }
    }
}

pub struct SymbolizedIndirectVarNodeDisplay {
    pub(crate) pointer_location: SymbolizedVarNodeDisplay,
    pub(crate) access_size_bytes: usize,
    pub(crate) pointer_space: SharedSpaceInfo,
}

impl Display for SymbolizedIndirectVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "*({}[{}]:{})",
            self.pointer_space.name, self.pointer_location, self.access_size_bytes
        )
    }
}

impl LowerHex for SymbolizedIndirectVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "*({}[{:x}]:{:x})",
            self.pointer_space.name, self.pointer_location, self.access_size_bytes
        )
    }
}

impl UpperHex for SymbolizedIndirectVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "*({}[{:X}]:{:X})",
            self.pointer_space.name, self.pointer_location, self.access_size_bytes
        )
    }
}

pub enum SymbolizedGeneralVarNodeDisplay {
    Direct(SymbolizedVarNodeDisplay),
    Indirect(SymbolizedIndirectVarNodeDisplay),
}

impl From<SymbolizedVarNodeDisplay> for SymbolizedGeneralVarNodeDisplay {
    fn from(value: SymbolizedVarNodeDisplay) -> Self {
        Self::Direct(value)
    }
}

impl From<SymbolizedIndirectVarNodeDisplay> for SymbolizedGeneralVarNodeDisplay {
    fn from(value: SymbolizedIndirectVarNodeDisplay) -> Self {
        Self::Indirect(value)
    }
}

impl Display for SymbolizedGeneralVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolizedGeneralVarNodeDisplay::Direct(d) => {
                write!(f, "{}", d)
            }
            SymbolizedGeneralVarNodeDisplay::Indirect(i) => {
                write!(f, "{}", i)
            }
        }
    }
}

impl LowerHex for SymbolizedGeneralVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolizedGeneralVarNodeDisplay::Direct(d) => {
                write!(f, "{:x}", d)
            }
            SymbolizedGeneralVarNodeDisplay::Indirect(i) => {
                write!(f, "{:x}", i)
            }
        }
    }
}

impl UpperHex for SymbolizedGeneralVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolizedGeneralVarNodeDisplay::Direct(d) => {
                write!(f, "{:X}", d)
            }
            SymbolizedGeneralVarNodeDisplay::Indirect(i) => {
                write!(f, "{:X}", i)
            }
        }
    }
}
