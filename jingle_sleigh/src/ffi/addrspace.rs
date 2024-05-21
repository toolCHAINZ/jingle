#[cxx::bridge]
pub(crate) mod bridge {
    #[rust_name = "SpaceType"]
    #[namespace = "ghidra"]
    #[derive(Debug, Hash, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
    #[repr(u32)]
    pub enum spacetype {
        ///< Special space to represent constants
        IPTR_CONSTANT = 0,
        ///< Normal spaces modelled by processor
        IPTR_PROCESSOR = 1,
        ///< addresses = offsets off of base register
        IPTR_SPACEBASE = 2,
        ///< Internally managed temporary space
        IPTR_INTERNAL = 3,
        ///< Special internal FuncCallSpecs reference
        IPTR_FSPEC = 4,
        ///< Special internal PcodeOp reference
        IPTR_IOP = 5,
        ///< Special virtual space to represent split variables
        IPTR_JOIN = 6,
    }

    unsafe extern "C++" {
        include!("jingle_sleigh/src/ffi/cpp/addrspace_handle.h");

        pub(crate) type AddrSpaceHandle;

        pub fn getName(&self) -> &str;

        pub fn getType(&self) -> SpaceType;
        pub fn getManager(&self) -> SharedPtr<AddrSpaceManagerHandle>;
        pub fn getWordSize(&self) -> u32;
        pub fn getAddrSize(&self) -> u32;
        pub fn getIndex(&self) -> i32;
        pub fn isBigEndian(&self) -> bool;
    }

    unsafe extern "C++" {
        #[namespace = "ghidra"]
        #[rust_name = "SpaceType"]
        type spacetype;
    }

    unsafe extern "C++" {
        include!("jingle_sleigh/src/ffi/cpp/addrspace_manager_handle.h");

        type AddrSpaceManagerHandle;

        fn getSpaceFromPointer(&self, i: u64) -> SharedPtr<AddrSpaceHandle>;
        fn getSpaceByIndex(&self, idx: i32) -> SharedPtr<AddrSpaceHandle>;
        fn getNumSpaces(&self) -> i32;
        fn getDefaultCodeSpace(&self) -> SharedPtr<AddrSpaceHandle>;
    }
}
