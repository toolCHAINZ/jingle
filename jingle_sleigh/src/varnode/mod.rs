use crate::error::JingleSleighError;
use std::borrow::Borrow;

use crate::SleighArchInfo;
use crate::ffi::instruction::bridge::VarnodeInfoFFI;
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
#[derive(Debug, Clone, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "pyo3", pyclass)]
pub struct VarNode {
    /// The index at which the relevant space can be found in a [`SleighArchInfo`]
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
    pub const CONST_SPACE_INDEX: usize = 0;

    pub fn is_const(&self) -> bool {
        self.space_index == Self::CONST_SPACE_INDEX
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
        let left = self_range.start <= other.start && self_range.end > other.start;
        let right = other.start <= self_range.start && other.end > self_range.start;
        left || right
    }

    pub fn min_offset(&self) -> u64 {
        self.offset
    }

    pub fn max_offset(&self) -> u64 {
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
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}[{}]:{}", self.space_index, self.offset, self.size)
    }
}

impl LowerHex for VarNode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{:x}[{:x}]:{:x}",
            self.space_index, self.offset, self.size
        )
    }
}

pub fn create_varnode<T: Borrow<SleighArchInfo>>(
    ctx: &T,
    name: &str,
    offset: u64,
    size: usize,
) -> Result<VarNode, JingleSleighError> {
    for (space_index, space) in ctx.borrow().spaces().iter().enumerate() {
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
#[derive(Debug, Clone, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IndirectVarNode {
    pub pointer_space_index: usize,
    pub pointer_location: VarNode,
    pub access_size_bytes: usize,
}

impl Display for IndirectVarNode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "*{}[ {} ]:{}",
            self.pointer_space_index, self.pointer_location, self.access_size_bytes
        )
    }
}

impl LowerHex for IndirectVarNode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "*{:x}[ {:x} ]:{:x}",
            self.pointer_space_index, self.pointer_location, self.access_size_bytes
        )
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GeneralizedVarNode {
    Direct(VarNode),
    Indirect(IndirectVarNode),
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
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            GeneralizedVarNode::Direct(v) => {
                write!(f, "{v}")
            }
            GeneralizedVarNode::Indirect(indirect) => {
                write!(f, "{indirect}")
            }
        }
    }
}

impl LowerHex for GeneralizedVarNode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            GeneralizedVarNode::Direct(v) => {
                write!(f, "{v:x}")
            }
            GeneralizedVarNode::Indirect(indirect) => {
                write!(f, "{indirect:x}")
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

    #[test]
    fn test_overlaps_true() {
        let vn1 = VarNode {
            offset: 0,
            space_index: 0,
            size: 4,
        };
        let vn2 = VarNode {
            offset: 2,
            space_index: 0,
            size: 4,
        };
        assert!(vn1.overlaps(&vn2));
        assert!(vn2.overlaps(&vn1));
    }

    #[test]
    fn test_overlaps_false_different_space() {
        let vn1 = VarNode {
            offset: 0,
            space_index: 0,
            size: 4,
        };
        let vn2 = VarNode {
            offset: 0,
            space_index: 1,
            size: 4,
        };
        assert!(!vn1.overlaps(&vn2));
        assert!(!vn2.overlaps(&vn1));
    }

    #[test]
    fn test_overlaps_false_no_overlap() {
        let vn1 = VarNode {
            offset: 0,
            space_index: 0,
            size: 4,
        };
        let vn2 = VarNode {
            offset: 10,
            space_index: 0,
            size: 4,
        };
        assert!(!vn1.overlaps(&vn2));
        assert!(!vn2.overlaps(&vn1));
    }

    #[test]
    fn test_covers_false_different_space() {
        let vn1 = VarNode {
            offset: 0,
            space_index: 0,
            size: 4,
        };
        let vn2 = VarNode {
            offset: 0,
            space_index: 1,
            size: 2,
        };
        assert!(!vn1.covers(&vn2));
    }

    #[test]
    fn test_covers_false_extends_beyond() {
        let vn1 = VarNode {
            offset: 0,
            space_index: 0,
            size: 4,
        };
        let vn2 = VarNode {
            offset: 2,
            space_index: 0,
            size: 4,
        };
        assert!(!vn1.covers(&vn2));
    }

    #[test]
    fn test_is_const() {
        let const_vn = VarNode {
            offset: 100,
            space_index: VarNode::CONST_SPACE_INDEX,
            size: 4,
        };
        assert!(const_vn.is_const());

        let non_const_vn = VarNode {
            offset: 100,
            space_index: 3,
            size: 4,
        };
        assert!(!non_const_vn.is_const());
    }

    #[test]
    fn test_min_max() {
        let vn = VarNode {
            offset: 100,
            space_index: 0,
            size: 8,
        };
        assert_eq!(vn.min_offset(), 100);
        assert_eq!(vn.max_offset(), 108);
    }

    #[test]
    fn test_range_conversion_u64() {
        let vn = VarNode {
            offset: 100,
            space_index: 0,
            size: 8,
        };
        let range: std::ops::Range<u64> = (&vn).into();
        assert_eq!(range.start, 100);
        assert_eq!(range.end, 108);
    }

    #[test]
    fn test_range_conversion_usize() {
        let vn = VarNode {
            offset: 100,
            space_index: 0,
            size: 8,
        };
        let range: std::ops::Range<usize> = (&vn).into();
        assert_eq!(range.start, 100);
        assert_eq!(range.end, 108);
    }

    #[test]
    fn test_overlaps_adjacent_ranges() {
        let vn1 = VarNode {
            offset: 0,
            space_index: 0,
            size: 4,
        };
        let vn2 = VarNode {
            offset: 4,
            space_index: 0,
            size: 4,
        };
        // Adjacent ranges should not overlap
        assert!(!vn1.overlaps(&vn2));
    }
}
