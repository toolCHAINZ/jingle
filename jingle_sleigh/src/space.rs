use crate::ffi::addrspace::bridge::SpaceType;
use crate::ffi::context_ffi::bridge::AddrSpaceHandle;
use crate::space::SleighEndianness::{Big, Little};
use crate::varnode::VarNode;
use cxx::SharedPtr;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

#[derive(Clone, PartialEq, Eq)]
/// A convenient cache of information about a sleigh context
pub(crate) struct SleighArchInfoInner {
    /// A mapping of register names to varnodes
    pub(crate) registers_to_vns: HashMap<String, VarNode>,
    /// A mapping of varnodes to register names
    pub(crate) vns_to_registers: HashMap<VarNode, String>,
    /// Ordered metadata about the spaces defined in this pcode context
    /// The order in this vector must match the ordering assumed
    /// in pcode operations
    pub(crate) spaces: Vec<SpaceInfo>,
    /// The index of pcode space in which code usually lives
    /// Used to interpret some pcode branch destinations, as well
    /// as in some varnode "helpers".
    ///
    /// On most platforms (e.g. not Harvard arch), this is just "ram"
    pub(crate) default_code_space: usize,
    /// A mapping from an index to the name associated with a `CALLOTHER`
    ///
    /// The first input varnode of a CALLOTHER is a constant, which can
    /// be used to index this map. This improves display of CALLOTHER as well
    /// as for parsing: users need not memorize CALLOTHER arguments.
    pub(crate) userops: Vec<String>,
}

impl std::fmt::Debug for SleighArchInfoInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SleighArchInfoInner")
            .field("registers_to_vns_count", &self.registers_to_vns.len())
            .field("vns_to_registers_count", &self.vns_to_registers.len())
            .field("spaces", &self.spaces)
            .field("default_code_space", &self.default_code_space)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SleighArchInfo {
    pub(crate) info: Arc<SleighArchInfoInner>,
}

impl SleighArchInfo {
    pub fn new<T: Iterator<Item = (VarNode, String)>, E: Iterator<Item = SpaceInfo>>(
        registers: T,
        spaces: E,
        default_code_space: usize,
        userops: Vec<String>,
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
                userops,
            }),
        }
    }

    pub fn get_space(&self, idx: usize) -> Option<&SpaceInfo> {
        self.info.spaces.get(idx)
    }

    pub fn get_space_by_name<T: AsRef<str>>(&self, t: T) -> Option<&SpaceInfo> {
        self.info.spaces.iter().find(|s| s.name.eq(t.as_ref()))
    }

    pub fn spaces(&self) -> &[SpaceInfo] {
        &self.info.spaces
    }

    pub fn registers(&self) -> impl Iterator<Item = (VarNode, String)> {
        self.info
            .vns_to_registers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
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

    pub fn varnode(&self, name: &str, offset: u64, size: usize) -> Option<VarNode> {
        let space_index = self.spaces().iter().position(|s| s.name == name)?;
        Some(VarNode {
            space_index,
            offset,
            size,
        })
    }

    /// Return the list of known userop names (by reference). Order is the
    /// canonical index order used by CALLOTHER operands.
    pub fn userops(&self) -> impl Iterator<Item = &String> {
        self.info.userops.iter()
    }

    /// Get the userop name for the given index, if it exists.
    pub fn userop_name(&self, idx: usize) -> Option<&str> {
        self.info.userops.get(idx).map(|s| s.as_str())
    }

    /// Find the index of a userop by name. Returns None if not found.
    pub fn userop_index<T: AsRef<str>>(&self, name: T) -> Option<usize> {
        let needle = name.as_ref();
        self.info.userops.iter().position(|s| s == needle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffi::addrspace::bridge::SpaceType;

    fn create_test_space_info(name: &str, index: usize, endianness: SleighEndianness) -> SpaceInfo {
        SpaceInfo {
            name: name.to_string(),
            index,
            index_size_bytes: 8,
            word_size_bytes: 1,
            _type: SpaceType::IPTR_PROCESSOR,
            endianness,
        }
    }

    #[test]
    fn test_space_info_make_varnode() {
        let space = create_test_space_info("ram", 3, SleighEndianness::Little);
        let vn = space.make_varnode(0x1000, 4);

        assert_eq!(vn.space_index, 3);
        assert_eq!(vn.offset, 0x1000);
        assert_eq!(vn.size, 4);
    }

    #[test]
    fn test_sleigh_arch_info_get_space() {
        let spaces = vec![
            create_test_space_info("const", 0, SleighEndianness::Little),
            create_test_space_info("unique", 1, SleighEndianness::Little),
            create_test_space_info("ram", 2, SleighEndianness::Little),
        ];

        let arch_info = SleighArchInfo::new(std::iter::empty(), spaces.into_iter(), 2, vec![]);

        let space = arch_info.get_space(1).unwrap();
        assert_eq!(space.name, "unique");
        assert_eq!(space.index, 1);

        assert!(arch_info.get_space(10).is_none());
    }

    #[test]
    fn test_sleigh_arch_info_get_space_by_name() {
        let spaces = vec![
            create_test_space_info("const", 0, SleighEndianness::Little),
            create_test_space_info("ram", 1, SleighEndianness::Little),
        ];

        let arch_info = SleighArchInfo::new(std::iter::empty(), spaces.into_iter(), 1, vec![]);

        let space = arch_info.get_space_by_name("ram").unwrap();
        assert_eq!(space.index, 1);

        assert!(arch_info.get_space_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_sleigh_arch_info_spaces() {
        let spaces = vec![
            create_test_space_info("const", 0, SleighEndianness::Little),
            create_test_space_info("ram", 1, SleighEndianness::Little),
        ];

        let arch_info = SleighArchInfo::new(std::iter::empty(), spaces.into_iter(), 1, vec![]);

        let all_spaces = arch_info.spaces();
        assert_eq!(all_spaces.len(), 2);
        assert_eq!(all_spaces[0].name, "const");
        assert_eq!(all_spaces[1].name, "ram");
    }

    #[test]
    fn test_sleigh_arch_info_registers() {
        let registers = vec![
            (
                VarNode {
                    space_index: 1,
                    offset: 0,
                    size: 8,
                },
                "rax".to_string(),
            ),
            (
                VarNode {
                    space_index: 1,
                    offset: 8,
                    size: 8,
                },
                "rbx".to_string(),
            ),
        ];

        let arch_info = SleighArchInfo::new(registers.into_iter(), std::iter::empty(), 1, vec![]);

        let regs: Vec<_> = arch_info.registers().collect();
        assert_eq!(regs.len(), 2);
    }

    #[test]
    fn test_sleigh_arch_info_register_name() {
        let rax_vn = VarNode {
            space_index: 1,
            offset: 0,
            size: 8,
        };
        let registers = vec![(rax_vn.clone(), "rax".to_string())];

        let arch_info = SleighArchInfo::new(registers.into_iter(), std::iter::empty(), 1, vec![]);

        assert_eq!(arch_info.register_name(&rax_vn), Some("rax"));

        let unknown_vn = VarNode {
            space_index: 1,
            offset: 100,
            size: 8,
        };
        assert_eq!(arch_info.register_name(&unknown_vn), None);
    }

    #[test]
    fn test_sleigh_arch_info_register() {
        let rax_vn = VarNode {
            space_index: 1,
            offset: 0,
            size: 8,
        };
        let registers = vec![(rax_vn.clone(), "rax".to_string())];

        let arch_info = SleighArchInfo::new(registers.into_iter(), std::iter::empty(), 1, vec![]);

        assert_eq!(arch_info.register("rax"), Some(&rax_vn));
        assert_eq!(arch_info.register("rbx"), None);
    }

    #[test]
    fn test_sleigh_arch_info_varnode() {
        let spaces = vec![
            create_test_space_info("ram", 0, SleighEndianness::Little),
            create_test_space_info("unique", 1, SleighEndianness::Little),
        ];

        let arch_info = SleighArchInfo::new(std::iter::empty(), spaces.into_iter(), 0, vec![]);

        let vn = arch_info.varnode("ram", 0x1000, 4).unwrap();
        assert_eq!(vn.space_index, 0);
        assert_eq!(vn.offset, 0x1000);
        assert_eq!(vn.size, 4);

        assert!(arch_info.varnode("nonexistent", 0, 4).is_none());
    }

    #[test]
    fn test_sleigh_arch_info_default_code_space_index() {
        let arch_info = SleighArchInfo::new(std::iter::empty(), std::iter::empty(), 3, vec![]);

        assert_eq!(arch_info.default_code_space_index(), 3);
    }

    #[test]
    fn test_sleigh_arch_info_userops() {
        let userops = vec!["syscall".to_string(), "cpuid".to_string()];

        let arch_info = SleighArchInfo::new(std::iter::empty(), std::iter::empty(), 0, userops);

        let ops: Vec<_> = arch_info.userops().collect();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0], "syscall");
        assert_eq!(ops[1], "cpuid");
    }

    #[test]
    fn test_sleigh_arch_info_userop_name() {
        let userops = vec!["syscall".to_string(), "cpuid".to_string()];

        let arch_info = SleighArchInfo::new(std::iter::empty(), std::iter::empty(), 0, userops);

        assert_eq!(arch_info.userop_name(0), Some("syscall"));
        assert_eq!(arch_info.userop_name(1), Some("cpuid"));
        assert_eq!(arch_info.userop_name(2), None);
    }

    #[test]
    fn test_sleigh_arch_info_userop_index() {
        let userops = vec!["syscall".to_string(), "cpuid".to_string()];

        let arch_info = SleighArchInfo::new(std::iter::empty(), std::iter::empty(), 0, userops);

        assert_eq!(arch_info.userop_index("syscall"), Some(0));
        assert_eq!(arch_info.userop_index("cpuid"), Some(1));
        assert_eq!(arch_info.userop_index("nonexistent"), None);
    }

    #[test]
    fn test_sleigh_endianness_equality() {
        assert_eq!(SleighEndianness::Big, SleighEndianness::Big);
        assert_eq!(SleighEndianness::Little, SleighEndianness::Little);
        assert_ne!(SleighEndianness::Big, SleighEndianness::Little);
    }

    #[test]
    fn test_space_info_equality() {
        let space1 = create_test_space_info("ram", 1, SleighEndianness::Little);
        let space2 = create_test_space_info("ram", 1, SleighEndianness::Little);
        let space3 = create_test_space_info("rom", 1, SleighEndianness::Little);

        assert_eq!(space1, space2);
        assert_ne!(space1, space3);
    }
}
