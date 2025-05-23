use crate::JingleSleighError;
use crate::ffi::addrspace::bridge::SpaceType;
use crate::ffi::context_ffi::bridge::AddrSpaceHandle;
use crate::space::SleighEndianness::{Big, Little};
use crate::varnode::VarNode;
use cxx::SharedPtr;
use serde::{Deserialize, Serialize};

/// What program-analysis library wouldn't be complete without an enum
/// for endianness?
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum SleighEndianness {
    Big,
    Little,
}

/// Information about a `PCODE` address space modeled by `SLEIGH`. Internally, `jingle` uses indices
/// to refer unambiguously and efficiently to spaces.
/// This has the advantage of drastically reducing the amount of alloc/drop churn when working with
/// `jingle` but has a cost: in order to use "nice" things like the names of spaces, you need to have
/// a way to refer to a [`SpaceInfo`] object.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct SpaceInfo {
    /// The name of the space; the name is guaranteed by `SLEIGH` to be unique, so it can be used
    /// as a unique identifier
    pub name: String,
    /// The index that this space occupies in `SLEIGH`'s table of spaces. Kind of redundant to have
    /// it in this struct, but this also allows for some convenience functions, so I'll allow it.
    pub index: usize,
    /// Spaces have an associated address size, here expressed in bytes
    pub index_size_bytes: u32,
    /// Spaces have an associated word size, here expressed in bytes. This will almost always
    /// be 1 byte per word.
    pub word_size_bytes: u32,
    /// `SLEIGH` models instructions using multiple spaces, some of which map directly to architectural
    /// spaces, others of which are internal `SLEIGH`-specific implementation details (e.g. the `const`
    /// space and the `unique` space). This tag allows for directly determining what role each
    /// space has.
    pub _type: SpaceType,
    /// What endianness to use when reading to/writing from this space. Varnode reads/writes are interpreted
    /// as using whatever endianness is set here
    pub endianness: SleighEndianness,
}

impl SpaceInfo {
    /// Create a varnode of the given offset and size residing in this space.
    pub fn make_varnode(&self, offset: u64, size: usize) -> VarNode {
        VarNode {
            space_index: self.index,
            offset,
            size,
        }
    }
}

impl From<AddrSpaceHandle> for SpaceInfo {
    fn from(value: AddrSpaceHandle) -> Self {
        Self {
            name: value.getName().to_string(),
            index: value.getIndex() as usize,
            index_size_bytes: value.getAddrSize(),
            word_size_bytes: value.getWordSize(),
            _type: value.getType(),
            endianness: match value.isBigEndian() {
                true => Big,
                false => Little,
            },
        }
    }
}

impl From<SharedPtr<AddrSpaceHandle>> for SpaceInfo {
    fn from(value: SharedPtr<AddrSpaceHandle>) -> Self {
        Self {
            name: value.getName().to_string(),
            index: value.getIndex() as usize,
            index_size_bytes: value.getAddrSize(),
            word_size_bytes: value.getWordSize(),
            _type: value.getType(),
            endianness: match value.isBigEndian() {
                true => Big,
                false => Little,
            },
        }
    }
}

/// `jingle` models traces of code using slices, so it is helpful to implement some of these
/// traits on slices of types that implement those same traits.
impl<T: ArchInfoProvider> ArchInfoProvider for &[T] {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self[0].get_space_info(idx)
    }

    fn get_all_space_info(&self) -> impl Iterator<Item = &SpaceInfo> {
        self[0].get_all_space_info()
    }

    fn get_code_space_idx(&self) -> usize {
        self[0].get_code_space_idx()
    }

    fn get_register(&self, name: &str) -> Option<&VarNode> {
        self[0].get_register(name)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self[0].get_register_name(location)
    }

    fn get_registers(&self) -> impl Iterator<Item = (&VarNode, &str)> {
        self[0].get_registers()
    }
}

pub trait ArchInfoProvider {
    /// Retrieve the [`SpaceInfo`] associated with the given index, if it exists
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo>;

    /// Retrieve a listing of all [`SpaceInfo`] associated with this `SLEIGH` context
    fn get_all_space_info(&self) -> impl Iterator<Item = &SpaceInfo>;

    /// Returns the index that `SLEIGH` claims is the "main" space in which instructions reside
    fn get_code_space_idx(&self) -> usize;

    /// Given a register name, get a corresponding [`VarNode`], if one exists
    fn get_register(&self, name: &str) -> Option<&VarNode>;

    /// Given a [`VarNode`], get the name of the corresponding architectural register, if one exists
    fn get_register_name(&self, location: &VarNode) -> Option<&str>;

    /// Get a listing of all register name/[`VarNode`] pairs
    fn get_registers(&self) -> impl Iterator<Item = (&VarNode, &str)>;

    fn varnode(&self, name: &str, offset: u64, size: usize) -> Result<VarNode, JingleSleighError> {
        for (space_index, space) in self.get_all_space_info().enumerate() {
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
}
