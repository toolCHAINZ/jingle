use std::collections::HashMap;
use crate::JingleSleighError;
use crate::ffi::addrspace::bridge::SpaceType;
use crate::ffi::context_ffi::bridge::AddrSpaceHandle;
use crate::space::SleighEndianness::{Big, Little};
use crate::varnode::VarNode;
use cxx::SharedPtr;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SleighArchInfoInner {
    pub(crate) registers_to_vns: HashMap<String, VarNode>,
    pub(crate) vns_to_registers: HashMap<VarNode, String>,
    pub(crate) spaces: Vec<SpaceInfo>,
    pub(crate) default_code_space: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SleighArchInfo {
    pub(crate) info: Arc<SleighArchInfoInner>,
}

impl SleighArchInfo {
    pub fn new<
        T: Iterator<Item=(VarNode, String)>,
        E: Iterator<Item=SpaceInfo>,
    >(
        registers: T,
        spaces: E,
        default_code_space: usize,
    ) -> Self {
        let mut registers_to_vns = HashMap::new();
        let mut vns_to_registers = HashMap::new();

        for (varnode, name) in registers {
            registers_to_vns.insert(name.clone(), varnode.clone());
            vns_to_registers.insert(varnode, name);
        }

        Self {
            info: Arc::new(SleighArchInfoInner {
                registers_to_vns,
                vns_to_registers,
                spaces: spaces.collect(),
                default_code_space,
            }),
        }
    }

    pub fn get_space(&self, idx: usize) -> Option<&SpaceInfo> {
        self.info.spaces.get(idx)
    }

    pub fn spaces(&self) -> &[SpaceInfo] {
        &self.info.spaces
    }

    pub fn registers(&self) -> impl Iterator<Item = (VarNode, String)> {
        self.info.vns_to_registers.iter().map(|(k, v)| (k.clone(), v.clone()))
    }

    pub fn default_code_space_index(&self) -> usize {
        self.info.default_code_space
    }
    pub fn register_name(&self, location: &VarNode) -> Option<&str> {
        self.info.vns_to_registers.get(location).map(|s| s.as_str())
    }

    pub fn register<T: AsRef<str>>(&self, name: T) -> Option<&VarNode> {
        self.info.registers_to_vns.get(name.as_ref())
    }
}