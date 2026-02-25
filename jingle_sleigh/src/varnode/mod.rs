use crate::error::JingleSleighError;
use std::borrow::Borrow;
use std::ops::Add;

use crate::SleighArchInfo;
use crate::ffi::instruction::bridge::VarnodeInfoFFI;
#[cfg(feature = "pyo3")]
use pyo3::pyclass;
#[cfg(feature = "pyo3")]
use pyo3::pymethods;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter, LowerHex};
use std::ops::Range;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VarNodeSize(u32);

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VarNodeSpaceIndex(u32);

impl Display for VarNodeSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Display for VarNodeSpaceIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl LowerHex for VarNodeSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

impl LowerHex for VarNodeSpaceIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        LowerHex::fmt(&self.0, f)
    }
}

macro_rules! into_vn_types {
    ($t:ident) => {
        impl From<$t> for VarNodeSize {
            fn from(value: $t) -> Self {
                Self(value as u32)
            }
        }

        impl From<$t> for VarNodeSpaceIndex {
            fn from(value: $t) -> Self {
                Self(value as u32)
            }
        }

        impl Add<$t> for VarNodeSize {
            type Output = $t;

            fn add(self, rhs: $t) -> Self::Output {
                (self.0 as $t) + rhs
            }
        }

        impl Add<VarNodeSize> for $t {
            type Output = $t;

            fn add(self, rhs: VarNodeSize) -> Self::Output {
                self + (rhs.0 as $t)
            }
        }
    };
}

into_vn_types!(u8);
into_vn_types!(u16);
into_vn_types!(u32);
into_vn_types!(i32);
into_vn_types!(u64);
into_vn_types!(usize);

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
    ///
    /// Use a compact integer because the number of spaces is small in practice.
    space_index: VarNodeSpaceIndex,
    /// The offset into the given space
    offset: u64,
    /// The size in bytes of the given [`VarNode`]
    ///
    /// todo: double-check the sleigh spec and see whether this is always bytes or if it is space word size
    size: VarNodeSize,
}

#[cfg(feature = "pyo3")]
#[pymethods]
impl VarNode {
    #[new]
    pub fn new_py(space_index: u32, offset: u64, size: u32) -> Self {
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
    pub const CONST_SPACE_INDEX: u32 = 0;

    pub fn new<I: Into<VarNodeSize>, J: Into<VarNodeSpaceIndex>>(
        offset: u64,
        size: I,
        space: J,
    ) -> Self {
        Self {
            offset,
            size: size.into(),
            space_index: space.into(),
        }
    }

    pub fn new_const<I: Into<VarNodeSize>>(offset: u64, size: I) -> Self {
        Self::new(offset, size, Self::CONST_SPACE_INDEX)
    }

    pub fn is_const(&self) -> bool {
        self.space_index == Self::CONST_SPACE_INDEX.into()
    }

    pub fn covers(&self, other: &VarNode) -> bool {
        if self.space_index != other.space_index {
            return false;
        }
        let self_range = self.offset..(self.offset + self.size);
        let other = other.offset..(other.offset + other.size);
        self_range.start <= other.start && self_range.end >= other.end
    }

    pub fn overlaps(&self, other: &VarNode) -> bool {
        if self.space_index != other.space_index {
            return false;
        }
        let self_range = self.offset..(self.offset + self.size);
        let other = other.offset..(other.offset + other.size);
        let left = self_range.start <= other.start && self_range.end > other.start;
        let right = other.start <= self_range.start && other.end > self_range.start;
        left || right
    }

    pub fn min_offset(&self) -> u64 {
        self.offset
    }

    pub fn max_offset(&self) -> u64 {
        self.offset + self.size
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn space_index(&self) -> usize {
        self.space_index.0 as usize
    }

    pub fn size(&self) -> usize {
        self.size.0 as usize
    }
}

impl From<&VarNode> for Range<u64> {
    fn from(value: &VarNode) -> Self {
        Range {
            start: value.offset,
            end: value.offset + value.size,
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
    size: u32,
) -> Result<VarNode, JingleSleighError> {
    for (space_index, space) in ctx.borrow().spaces().iter().enumerate() {
        if space.name.eq(name) {
            return Ok(VarNode::new(offset, size, space_index));
        }
    }
    Err(JingleSleighError::InvalidSpaceName)
}

#[cfg_attr(feature = "pyo3", pyclass)]
#[derive(Debug, Clone, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IndirectVarNode {
    pointer_space_index: VarNodeSpaceIndex,
    pointer_location: VarNode,
    access_size_bytes: VarNodeSize,
}

impl IndirectVarNode {
    pub fn new(
        pointer: impl Borrow<VarNode>,
        size: impl Into<VarNodeSize>,
        space: impl Into<VarNodeSpaceIndex>,
    ) -> Self {
        Self {
            pointer_location: pointer.borrow().clone(),
            access_size_bytes: size.into(),
            pointer_space_index: space.into(),
        }
    }

    pub fn access_size_bytes(&self) -> usize {
        self.access_size_bytes.0 as usize
    }

    pub fn pointer_location(&self) -> &VarNode {
        &self.pointer_location
    }

    pub fn pointer_space_index(&self) -> usize {
        self.pointer_space_index.0 as usize
    }

    pub fn set_access_size_bytes(&mut self, val: impl Into<VarNodeSize>) {
        self.access_size_bytes = val.into()
    }
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
            size: value.size.into(),
            space_index: (value.space.getIndex() as u32).into(),
            offset: value.offset,
        }
    }
}

impl From<&VarnodeInfoFFI> for VarNode {
    fn from(value: &VarnodeInfoFFI) -> Self {
        Self {
            size: value.size.into(),
            space_index: (value.space.getIndex() as u32).into(),
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
        let vn1 = VarNode::new(0u64, 4u32, 0u32);
        let tests = [
            VarNode::new(0u64, 4u32, 0u32),
            VarNode::new(0u64, 3u32, 0u32),
            VarNode::new(0u64, 2u32, 0u32),
            VarNode::new(2u64, 1u32, 0u32),
            VarNode::new(2u64, 2u32, 0u32),
            VarNode::new(2u64, 1u32, 0u32),
        ];
        assert!(tests.iter().all(|v| vn1.covers(v)))
    }

    #[test]
    fn test_overlaps_true() {
        let vn1 = VarNode::new(0u64, 4u32, 0u32);
        let vn2 = VarNode::new(2u64, 4u32, 0u32);
        assert!(vn1.overlaps(&vn2));
        assert!(vn2.overlaps(&vn1));
    }

    #[test]
    fn test_overlaps_false_different_space() {
        let vn1 = VarNode::new(0u64, 4u32, 0u32);
        let vn2 = VarNode::new(0u64, 4u32, 1u32);
        assert!(!vn1.overlaps(&vn2));
        assert!(!vn2.overlaps(&vn1));
    }

    #[test]
    fn test_overlaps_false_no_overlap() {
        let vn1 = VarNode::new(0u64, 4u32, 0u32);
        let vn2 = VarNode::new(10u64, 4u32, 0u32);
        assert!(!vn1.overlaps(&vn2));
        assert!(!vn2.overlaps(&vn1));
    }

    #[test]
    fn test_covers_false_different_space() {
        let vn1 = VarNode::new(0u64, 4u32, 0u32);
        let vn2 = VarNode::new(0u64, 2u32, 1u32);
        assert!(!vn1.covers(&vn2));
    }

    #[test]
    fn test_covers_false_extends_beyond() {
        let vn1 = VarNode::new(0u64, 4u32, 0u32);
        let vn2 = VarNode::new(2u64, 4u32, 0u32);
        assert!(!vn1.covers(&vn2));
    }

    #[test]
    fn test_is_const() {
        let const_vn = VarNode::new_const(100u64, 4u32);
        assert!(const_vn.is_const());

        let non_const_vn = VarNode::new(100u64, 4u32, 3u32);
        assert!(!non_const_vn.is_const());
    }

    #[test]
    fn test_min_max() {
        let vn = VarNode::new(100u64, 8u32, 0u32);
        assert_eq!(vn.min_offset(), 100);
        assert_eq!(vn.max_offset(), 108);
    }

    #[test]
    fn test_range_conversion_u64() {
        let vn = VarNode::new(100u64, 8u32, 0u32);
        let range: std::ops::Range<u64> = (&vn).into();
        assert_eq!(range.start, 100);
        assert_eq!(range.end, 108);
    }

    #[test]
    fn test_range_conversion_usize() {
        let vn = VarNode::new(100u64, 8u32, 0u32);
        let range: std::ops::Range<usize> = (&vn).into();
        assert_eq!(range.start, 100);
        assert_eq!(range.end, 108);
    }

    #[test]
    fn test_overlaps_adjacent_ranges() {
        let vn1 = VarNode::new(0u64, 4u32, 0u32);
        let vn2 = VarNode::new(4u64, 4u32, 0u32);
        // Adjacent ranges should not overlap
        assert!(!vn1.overlaps(&vn2));
    }
}
