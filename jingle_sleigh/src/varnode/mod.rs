pub mod display;

use crate::error::JingleSleighError;

use crate::ffi::instruction::bridge::VarnodeInfoFFI;
pub use crate::varnode::display::{
    GeneralizedVarNodeDisplay, IndirectVarNodeDisplay, VarNodeDisplay,
};
use crate::{ArchInfoProvider, RawVarNodeDisplay};
#[cfg(feature = "pyo3")]
use pyo3::pyclass;
#[cfg(feature = "pyo3")]
use pyo3::pymethods;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter, LowerHex};
use std::ops::Range;

/// A [`VarNode`] is `SLEIGH`'s generalization of an address. It describes a sized-location in
/// a given memory space.
///
/// This is the fundamental data type of `PCODE` operations, and is used to encode all data inputs
/// and outputs of the instruction semantics.
///
/// In `jingle`, we follow `SLEIGH`'s convention and display these as
/// `<space>\[<offset>\]:<size>`. In the case of constants, we simplify this to `<offset>:<size>`.
/// For registers, we will (soon! (TM)) perform a register lookup and instead show the pretty
/// architecture-defined register name.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "pyo3", pyclass)]
pub struct VarNode {
    /// The index at which the relevant space can be found in a [`ArchInfoProvider`]
    pub space_index: usize,
    /// The offset into the given space
    pub offset: u64,
    /// The size in bytes of the given [`VarNode`]
    ///
    /// todo: double-check the sleigh spec and see whether this is always bytes or if it is space word size
    pub size: usize,
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl VarNode {
    #[new]
    pub fn new(space_index: usize, offset: u64, size: usize) -> Self {
        Self {
            space_index,
            offset,
            size,
        }
    }
}

impl VarNode {
    /// This value is hardcoded in `space.cc` within `SLEIGH`. Also hardcoding it here for convenience.
    /// todo: It would be best if this was checked with a static assert from cxx
    const CONST_SPACE_INDEX: usize = 0;

    pub fn is_const(&self) -> bool {
        self.space_index == Self::CONST_SPACE_INDEX
    }
    pub fn display<T: ArchInfoProvider>(
        &self,
        ctx: &T,
    ) -> Result<VarNodeDisplay, JingleSleighError> {
        if let Some(name) = ctx.get_register_name(self) {
            Ok(VarNodeDisplay::Register(name.to_string(), self.clone()))
        } else {
            ctx.get_space_info(self.space_index)
                .map(|space_info| {
                    VarNodeDisplay::Raw(RawVarNodeDisplay {
                        size: self.size,
                        offset: self.offset,
                        space_info: space_info.clone(),
                    })
                })
                .ok_or(JingleSleighError::InvalidSpaceName)
        }
    }

    pub fn covers(&self, other: &VarNode) -> bool {
        if self.space_index != other.space_index {
            return false;
        }
        let self_range = self.offset..(self.offset + self.size as u64);
        let other = other.offset..(other.offset + other.size as u64);
        self_range.start <= other.start && self_range.end >= other.end
    }

    pub fn overlaps(&self, other: &VarNode) -> bool {
        if self.space_index != other.space_index {
            return false;
        }
        let self_range = self.offset..(self.offset + self.size as u64);
        let other = other.offset..(other.offset + other.size as u64);
        let left = self_range.start <= other.start && self_range.end >= other.start;
        let right = other.start <= self_range.start && other.end >= self_range.start;
        left || right
    }

    pub fn min(&self) -> u64 {
        self.offset
    }

    pub fn max(&self) -> u64 {
        self.offset + self.size as u64
    }
}

impl From<&VarNode> for Range<u64> {
    fn from(value: &VarNode) -> Self {
        Range {
            start: value.offset,
            end: value.offset + value.size as u64,
        }
    }
}

impl From<&VarNode> for Range<usize> {
    fn from(value: &VarNode) -> Self {
        Range {
            start: value.offset as usize,
            end: value.offset as usize + value.size,
        }
    }
}

impl Display for VarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]:{}", self.space_index, self.offset, self.size)
    }
}

impl LowerHex for VarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:x}[{:x}]:{:x}",
            self.space_index, self.offset, self.size
        )
    }
}

#[macro_export]
macro_rules! varnode {
    ($ctx:expr_2021, #$offset:literal:$size:literal) => {
        $ctx.varnode("const", $offset, $size)
    };
    ($ctx:expr_2021, $space:literal[$offset:expr_2021]:$size:literal) => {
        $ctx.varnode($space, $offset, $size)
    };
}

pub fn create_varnode<T: ArchInfoProvider>(
    ctx: &T,
    name: &str,
    offset: u64,
    size: usize,
) -> Result<VarNode, JingleSleighError> {
    for (space_index, space) in ctx.get_all_space_info().enumerate() {
        if space.name.eq(name) {
            return Ok(VarNode {
                space_index,
                size,
                offset,
            });
        }
    }
    Err(JingleSleighError::InvalidSpaceName)
}

#[cfg_attr(feature = "pyo3", pyclass)]
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct IndirectVarNode {
    pub pointer_space_index: usize,
    pub pointer_location: VarNode,
    pub access_size_bytes: usize,
}

impl IndirectVarNode {
    pub fn display<T: ArchInfoProvider>(
        &self,
        ctx: &T,
    ) -> Result<IndirectVarNodeDisplay, JingleSleighError> {
        let pointer_location = self.pointer_location.display(ctx);
        let pointer_space_name = ctx
            .get_space_info(self.pointer_space_index)
            .ok_or(JingleSleighError::InvalidSpaceName);
        pointer_location.and_then(|pointer_loc| {
            pointer_space_name.map(|space| IndirectVarNodeDisplay {
                pointer_space_info: space.clone(),
                pointer_location: pointer_loc,
                access_size_bytes: self.access_size_bytes,
            })
        })
    }
}

impl Display for IndirectVarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "*{}[ {} ]:{}",
            self.pointer_space_index, self.pointer_location, self.access_size_bytes
        )
    }
}

impl LowerHex for IndirectVarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "*{:x}[ {:x} ]:{:x}",
            self.pointer_space_index, self.pointer_location, self.access_size_bytes
        )
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum GeneralizedVarNode {
    Direct(VarNode),
    Indirect(IndirectVarNode),
}

impl GeneralizedVarNode {
    pub fn display<T: ArchInfoProvider>(
        &self,
        ctx: &T,
    ) -> Result<GeneralizedVarNodeDisplay, JingleSleighError> {
        match self {
            GeneralizedVarNode::Direct(d) => Ok(GeneralizedVarNodeDisplay::Direct(d.display(ctx)?)),
            GeneralizedVarNode::Indirect(i) => {
                Ok(GeneralizedVarNodeDisplay::Indirect(i.display(ctx)?))
            }
        }
    }
}

impl From<&VarNode> for GeneralizedVarNode {
    fn from(value: &VarNode) -> Self {
        GeneralizedVarNode::Direct(value.clone())
    }
}

impl From<&IndirectVarNode> for GeneralizedVarNode {
    fn from(value: &IndirectVarNode) -> Self {
        GeneralizedVarNode::Indirect(value.clone())
    }
}

impl From<VarNode> for GeneralizedVarNode {
    fn from(value: VarNode) -> Self {
        GeneralizedVarNode::Direct(value)
    }
}

impl From<IndirectVarNode> for GeneralizedVarNode {
    fn from(value: IndirectVarNode) -> Self {
        GeneralizedVarNode::Indirect(value)
    }
}

impl From<VarnodeInfoFFI> for VarNode {
    fn from(value: VarnodeInfoFFI) -> Self {
        Self {
            size: value.size,
            space_index: value.space.getIndex() as usize,
            offset: value.offset,
        }
    }
}

impl From<&VarnodeInfoFFI> for VarNode {
    fn from(value: &VarnodeInfoFFI) -> Self {
        Self {
            size: value.size,
            space_index: value.space.getIndex() as usize,
            offset: value.offset,
        }
    }
}

impl Display for GeneralizedVarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneralizedVarNode::Direct(v) => {
                write!(f, "{}", v)
            }
            GeneralizedVarNode::Indirect(indirect) => {
                write!(f, "{}", indirect)
            }
        }
    }
}

impl LowerHex for GeneralizedVarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneralizedVarNode::Direct(v) => {
                write!(f, "{:x}", v)
            }
            GeneralizedVarNode::Indirect(indirect) => {
                write!(f, "{:x}", indirect)
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::VarNode;

    #[test]
    fn test_overlap() {
        let vn1 = VarNode {
            offset: 0,
            space_index: 0,
            size: 4,
        };
        let tests = [
            VarNode {
                offset: 0,
                space_index: 0,
                size: 4,
            },
            VarNode {
                offset: 0,
                space_index: 0,
                size: 3,
            },
            VarNode {
                offset: 0,
                space_index: 0,
                size: 2,
            },
            VarNode {
                offset: 2,
                space_index: 0,
                size: 1,
            },
            VarNode {
                offset: 2,
                space_index: 0,
                size: 2,
            },
            VarNode {
                offset: 2,
                space_index: 0,
                size: 1,
            },
        ];
        assert!(tests.iter().all(|v| vn1.covers(v)))
    }
}
