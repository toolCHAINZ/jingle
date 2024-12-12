pub mod display;
mod pod;

use crate::space::SharedSpaceInfo;
use crate::SpaceType;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Range;
use crate::varnode::pod::PodVarNode;

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
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct VarNode {
    /// The index at which the relevant space can be found in a [`SpaceManager`]
    pub space: SharedSpaceInfo,
    /// The offset into the given space
    pub offset: u64,
    /// The size in bytes of the given [`VarNode`]
    ///
    /// todo: double-check the sleigh spec and see whether this is always bytes or if it is space word size
    pub size: usize,
}

impl VarNode {
    pub fn is_const(&self) -> bool {
        self.space._type == SpaceType::IPTR_CONSTANT
    }

    pub fn covers(&self, other: &VarNode) -> bool {
        if self.space.index != other.space.index {
            return false;
        }
        let self_range = self.offset..(self.offset + self.size as u64);
        let other = other.offset..(other.offset + other.size as u64);
        self_range.start <= other.start && self_range.end >= other.end
    }
    pub fn to_pod(&self) -> PodVarNode{
        self.into()
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
#[macro_export]
macro_rules! varnode {
    ($ctx:expr, #$offset:literal:$size:literal) => {
        $ctx.varnode("const", $offset, $size)
    };
    ($ctx:expr, $space:literal[$offset:expr]:$size:literal) => {
        $ctx.varnode($space, $offset, $size)
    };
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct IndirectVarNode {
    pub pointer_space: SharedSpaceInfo,
    pub pointer_location: VarNode,
    pub access_size_bytes: usize,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
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

#[cfg(test)]
mod tests {
    use crate::space::SharedSpaceInfo;
    use crate::{SleighEndianness, SpaceInfo, SpaceType, VarNode};
    use std::rc::Rc;

    #[test]
    fn test_overlap() {
        let space: SharedSpaceInfo = Rc::new(SpaceInfo {
            index: 3,
            index_size_bytes: 4,
            word_size_bytes: 4,
            _type: SpaceType::IPTR_PROCESSOR,
            name: "ram".to_string(),
            endianness: SleighEndianness::Little,
        })
        .into();

        let vn1 = VarNode {
            offset: 0,
            space: space.clone(),
            size: 4,
        };
        let tests = [
            VarNode {
                offset: 0,
                space: space.clone(),
                size: 4,
            },
            VarNode {
                offset: 0,
                space: space.clone(),
                size: 3,
            },
            VarNode {
                offset: 0,
                space: space.clone(),
                size: 2,
            },
            VarNode {
                offset: 2,
                space: space.clone(),
                size: 1,
            },
            VarNode {
                offset: 2,
                space: space.clone(),
                size: 2,
            },
            VarNode {
                offset: 2,
                space: space.clone(),
                size: 1,
            },
        ];
        assert!(tests.iter().all(|v| vn1.covers(v)))
    }
}
