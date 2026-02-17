mod builder;
pub mod image;
mod instruction_iterator;
pub mod loaded;
mod python;

use crate::error::JingleSleighError;
use crate::error::JingleSleighError::{LanguageSpecRead, SleighInitError};
use crate::ffi::addrspace::bridge::AddrSpaceHandle;
use crate::ffi::context_ffi::bridge::ContextFFI;
use crate::parse::parse_program;
use crate::space::{SleighArchInfo, SleighArchInfoInner, SpaceInfo};
pub use builder::SleighContextBuilder;
use std::collections::HashMap;

use crate::JingleSleighError::{ImageLoadError, SleighCompilerMutexError};
use crate::context::builder::language_def::Language;
use crate::context::loaded::LoadedSleighContext;
use crate::ffi::context_ffi::CTX_BUILD_MUTEX;
use crate::{PcodeOperation, VarNode};
use image::SleighImage;
use cxx::{SharedPtr, UniquePtr};
#[cfg(feature = "pyo3")]
use pyo3::pyclass;
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

#[non_exhaustive]
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideEffect {
    /// Increment the given register by a given amount
    /// Decrement the given register by a given amount
    RegisterIncrement(String, u8),
    RegisterDecrement(String, u8),
}

pub struct ModelingSummary {}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
/// A flag indicating how to model a function call
pub enum ModelingBehavior {
    /// Treat this function call as a branch to some terminating
    /// piece of code
    Terminate,
    /// This function call should be inlined directly into the CFG during
    /// modeling (still a todo, will require restructuring built CFGs)
    Inline,
    /// The default behavior: model the side-effects of a function with a
    /// user-supplied set of side-effects
    Summary(Vec<SideEffect>),
}

impl Default for ModelingBehavior {
    fn default() -> Self {
        Self::Summary(Vec::new())
    }
}

/// A naive representation of the effects of a function
#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "pyo3", pyclass)]
pub struct CallInfo {
    /// Argument varnodes associated with this call (if known from signature metadata)
    pub args: Vec<VarNode>,
    /// Optional output varnodes (for functions returning via memory or hidden return)
    pub outputs: Option<Vec<VarNode>>,
    /// How to model the call's side-effects by default
    pub model_behavior: ModelingBehavior,
    /// Optional extrapop value (stack purge) associated with this call site.
    /// When present, this should override any default extrapop derived from
    /// calling-convention prototypes for this specific call.
    pub extrapop: Option<i32>,
    /// Registers (as varnodes) clobbered by this call site. Empty when unknown.
    /// This is populated from calling-convention prototype `killedbycall` lists
    /// when available and enriched during instruction postprocessing.
    pub killed_regs: Vec<VarNode>,
}

#[derive(Debug, Clone, Default)]
pub struct ModelingMetadata {
    pub(crate) func_info: HashMap<u64, CallInfo>,
    pub(crate) callother_info: HashMap<Vec<VarNode>, CallInfo>,
}

impl ModelingMetadata {
    pub(crate) fn add_call_def(&mut self, addr: u64, info: CallInfo) {
        self.func_info.insert(addr, info);
    }
    pub(crate) fn add_callother_def(&mut self, sig: &[VarNode], info: CallInfo) {
        self.callother_info.insert(sig.to_vec(), info);
    }
}

/// A sleigh context contains the parsed sleigh state as well as
/// modeling metadata for analysis consumers.
///
/// Additional types used to capture calling-convention and compiler-spec
/// metadata parsed from .cspec files.
///
/// We model a small subset of the compiler spec relevant to calling conventions:
/// - `PrototypeInfo` represents a declared prototype and includes extracted
///   `extrapop` and `stackshift` as well as the parsed argument entries
///   (`pentries`) and lists of registers/varnodes that are `killedbycall` or
///   listed as `unaffected`.
/// - `PentryInfo` models `<pentry ...>` elements within a prototype.
/// - `CompilerSpecInfo` records discovered .cspec files for the language.
#[derive(Clone, Debug)]
pub struct PentryInfo {
    /// Optional minimum size (from `minsize` attribute)
    pub minsize: Option<u32>,
    /// Optional maximum size (from `maxsize` attribute)
    pub maxsize: Option<u32>,
    /// Optional alignment (from `align` attribute)
    pub align: Option<u32>,
    /// Optional storage string (e.g., "float", "hiddenret", "pointer", etc.)
    pub storage: Option<String>,
    /// Registers mentioned in this pentry (if any)
    pub registers: Vec<String>,
    /// If this pentry refers to a stack/address entry, captures the address space
    /// string (e.g., "stack") and an optional offset
    pub addr_space: Option<String>,
    pub addr_offset: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct PrototypeInfo {
    /// The prototype name (e.g., "__stdcall", "MSABI", etc.)
    pub name: String,
    /// Optional extrapop value (amount popped by callee). Unknown is represented by None.
    pub extrapop: Option<i32>,
    /// Optional stackshift value (stack shift for this prototype). Unknown is represented by None.
    pub stackshift: Option<i32>,
    /// The parsed pentry list (arguments / stack entries). May be empty.
    pub pentries: Vec<PentryInfo>,
    /// Registers (by name) that are killed by the call.
    pub killed_by_call: Vec<String>,
    /// Registers / varnodes listed as unaffected by the call.
    pub unaffected: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CompilerSpecInfo {
    /// Path (resolved) to the .cspec file used to derive this info
    pub path: std::path::PathBuf,
    /// Optional human-friendly name (if present in the file / language metadata)
    pub name: Option<String>,
    /// Whether this spec was designated as the default for the language
    pub is_default: bool,
}

/// Ref-counted container for calling-convention-related metadata.
///
/// This mirrors the style used for `SleighArchInfo` and allows callers to cheaply
/// clone a handle to all discovered compiler-spec and prototype information.
#[derive(Clone, Debug)]
pub(crate) struct CallingConventionInfoInner {
    pub(crate) compiler_specs: Vec<CompilerSpecInfo>,
    pub(crate) default_compiler_spec: Option<CompilerSpecInfo>,
    pub(crate) call_conventions: Vec<PrototypeInfo>,
    pub(crate) default_calling_convention: Option<PrototypeInfo>,
}

#[derive(Clone, Debug)]
pub struct CallingConventionInfo {
    pub(crate) info: std::sync::Arc<CallingConventionInfoInner>,
}

impl Default for CallingConventionInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl CallingConventionInfo {
    pub fn new() -> Self {
        Self {
            info: std::sync::Arc::new(CallingConventionInfoInner {
                compiler_specs: Vec::new(),
                default_compiler_spec: None,
                call_conventions: Vec::new(),
                default_calling_convention: None,
            }),
        }
    }

    pub fn compiler_specs(&self) -> &Vec<CompilerSpecInfo> {
        &self.info.compiler_specs
    }

    pub fn default_compiler_spec(&self) -> Option<&CompilerSpecInfo> {
        self.info.default_compiler_spec.as_ref()
    }

    pub fn call_conventions(&self) -> &Vec<PrototypeInfo> {
        &self.info.call_conventions
    }

    pub fn default_calling_convention(&self) -> Option<&PrototypeInfo> {
        self.info.default_calling_convention.as_ref()
    }
}

/// A sleigh context contains the parsed sleigh state as well as
/// modeling metadata for analysis consumers.
pub struct SleighContext {
    /// The FFI context handle wrapped in a Mutex for thread-safety.
    /// The underlying Sleigh C++ library is not thread-safe, so all access
    /// to ctx must be synchronized.
    pub(crate) ctx: Mutex<UniquePtr<ContextFFI>>,
    language_id: String,
    arch_info: SleighArchInfo,
    pub(crate) metadata: ModelingMetadata,
    /// Ref-counted container that holds compiler-spec and calling-convention info
    pub(crate) calling_convention_info: CallingConventionInfo,
}

unsafe impl Send for SleighContext {}
unsafe impl Sync for SleighContext {}

impl Debug for SleighContext {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Sleigh {{arch: {}}}", self.language_id)
    }
}

impl SleighContext {
    pub(crate) fn new<T: AsRef<Path>>(
        language_def: &Language,
        base_path: T,
    ) -> Result<Self, JingleSleighError> {
        let path = base_path.as_ref().join(&language_def.sla_file);
        let abs = path.canonicalize().map_err(|_| LanguageSpecRead)?;
        let path_str = abs.to_str().ok_or(LanguageSpecRead)?;
        match CTX_BUILD_MUTEX.lock() {
            Ok(make_context) => {
                let ctx = make_context(path_str).map_err(|e| SleighInitError(e.to_string()))?;
                let mut spaces: Vec<SpaceInfo> = Vec::with_capacity(ctx.getNumSpaces() as usize);
                for idx in 0..ctx.getNumSpaces() {
                    spaces.push(SpaceInfo::from(ctx.getSpaceByIndex(idx)));
                }
                let mut registers_to_vns = HashMap::new();
                let mut vns_to_registers = HashMap::new();

                for info in ctx.getRegisters() {
                    let vn = VarNode::from(info.varnode);
                    registers_to_vns.insert(info.name.clone(), vn.clone());
                    vns_to_registers.insert(vn, info.name);
                }

                let userops = ctx.getUserOps();

                let arch_info = SleighArchInfo {
                    info: Arc::new(SleighArchInfoInner {
                        language_id: language_def.id.clone(),
                        registers_to_vns,
                        vns_to_registers,
                        // todo: this is weird, should probably clean up
                        // this api
                        default_code_space: ctx
                            .getSpaceByIndex(0)
                            .getManager()
                            .getDefaultCodeSpace()
                            .getIndex() as usize,
                        spaces: spaces.clone(),
                        userops,
                        stack_pointer: None,
                        program_counter: None,
                    }),
                };

                Ok(Self {
                    ctx: Mutex::new(ctx),
                    arch_info,
                    language_id: language_def.id.clone(),
                    metadata: Default::default(),
                    calling_convention_info: CallingConventionInfo::new(),
                })
            }
            Err(_) => Err(SleighCompilerMutexError),
        }
    }

    pub(crate) fn set_initial_context(
        &mut self,
        name: &str,
        value: u32,
    ) -> Result<(), JingleSleighError> {
        self.ctx
            .lock()
            .unwrap()
            .pin_mut()
            .set_initial_context(name, value)
            .map_err(|_| ImageLoadError)
    }

    pub fn spaces(&self) -> Vec<SharedPtr<AddrSpaceHandle>> {
        let ctx = self.ctx.lock().unwrap();
        let mut spaces = Vec::with_capacity(ctx.getNumSpaces() as usize);
        for i in 0..ctx.getNumSpaces() {
            spaces.push(ctx.getSpaceByIndex(i))
        }
        spaces
    }

    pub fn get_language_id(&self) -> &str {
        &self.language_id
    }

    pub fn arch_info(&self) -> &SleighArchInfo {
        &self.arch_info
    }

    pub fn add_call_metadata(&mut self, addr: u64, info: CallInfo) {
        self.metadata.add_call_def(addr, info);
    }

    pub fn add_callother_metadata(&mut self, sig: &[VarNode], info: CallInfo) {
        self.metadata.add_callother_def(sig, info);
    }

    /// Return a reference to the ref-counted calling-convention info.
    pub fn calling_convention_info(&self) -> &CallingConventionInfo {
        &self.calling_convention_info
    }

    /// Replace the calling-convention info for this Sleigh context.
    pub fn set_calling_convention_info(&mut self, info: CallingConventionInfo) {
        self.calling_convention_info = info;
    }

    /// Convenience accessor for default stack change derived from the default calling convention.
    pub fn default_stack_change(&self) -> Option<i32> {
        if let Some(proto) = self.calling_convention_info.default_calling_convention() {
            if let Some(extrapop) = proto.extrapop {
                let stackshift = proto.stackshift.unwrap_or(0);
                return Some(extrapop - stackshift);
            }
        }
        None
    }

    pub fn parse_pcode_listing<T: AsRef<str>>(
        &self,
        s: T,
    ) -> Result<Vec<PcodeOperation>, JingleSleighError> {
        parse_program(s, &self.arch_info)
    }

    pub fn initialize_with_image<'b, T: SleighImage + 'b>(
        self,
        img: T,
    ) -> Result<LoadedSleighContext<'b>, JingleSleighError> {
        LoadedSleighContext::new(self, img)
    }

    /// Set the stack pointer varnode in the arch info.
    /// This replaces or sets the stack_pointer entry in the cached arch info.
    pub(crate) fn set_stack_pointer_varnode(&mut self, vn: VarNode) {
        let inner = Arc::make_mut(&mut self.arch_info.info);
        inner.stack_pointer = Some(vn);
    }

    /// Set the program counter varnode in the arch info.
    pub(crate) fn set_program_counter_varnode(&mut self, vn: VarNode) {
        let inner = Arc::make_mut(&mut self.arch_info.info);
        inner.program_counter = Some(vn);
    }
}

impl AsRef<SleighArchInfo> for SleighContext {
    fn as_ref(&self) -> &SleighArchInfo {
        self.arch_info()
    }
}

#[cfg(test)]
mod test {
    use crate::context::SleighContextBuilder;
    use crate::tests::SLEIGH_ARCH;
    use crate::{OpCode, VarNode};

    #[test]
    fn get_regs() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        let regs: Vec<_> = sleigh.arch_info().registers().collect();
        assert!(!regs.is_empty());
    }

    #[test]
    fn get_register_name() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        for (vn, name) in sleigh.arch_info().registers() {
            let addr = sleigh.as_ref().register(name);
            assert_eq!(addr, Some(&vn));
        }
    }

    #[test]
    fn get_invalid_register_name() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        assert_eq!(sleigh.arch_info().register("fake"), None);
    }

    #[test]
    fn get_valid_register() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();

        assert_eq!(
            sleigh.arch_info().register_name(&VarNode {
                space_index: 4,
                offset: 512,
                size: 1
            }),
            Some("CF")
        );
    }

    #[test]
    fn get_invalid_register() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();

        assert_eq!(
            sleigh.arch_info().register_name(&VarNode {
                space_index: 40,
                offset: 5122,
                size: 1
            }),
            None
        );
    }

    #[test]
    fn get_user_ops() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();

        // Access the userops vector stored in the arch info. This is a placeholder
        // assertion; replace with concrete expectations later.
        let name = sleigh.arch_info().userop_name(0);
        // dummy assertion to ensure the API was called and returned a Vec
        assert_eq!(name, Some("segment"));
    }

    #[test]
    fn load_slice() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();

        // an x86 push
        let img = vec![0x55u8];
        let sleigh = sleigh.initialize_with_image(img).unwrap();
        let instr = sleigh.instruction_at(0).unwrap();
        assert_eq!(instr.disassembly.mnemonic, "PUSH");
        assert_eq!(instr.ops.len(), 4); // extra op because we emit "fallthrough" branches now
        // the stages of a push in pcode
        assert_eq!(instr.ops[0].opcode(), OpCode::CPUI_COPY);
        assert_eq!(instr.ops[1].opcode(), OpCode::CPUI_INT_SUB);
        assert_eq!(instr.ops[2].opcode(), OpCode::CPUI_STORE);
    }
}
