mod builder;
pub mod image;
mod instruction_iterator;
pub mod loaded;

use crate::error::JingleSleighError;
use crate::error::JingleSleighError::{LanguageSpecRead, SleighInitError};
use crate::ffi::addrspace::bridge::AddrSpaceHandle;
use crate::ffi::context_ffi::bridge::ContextFFI;
use crate::space::{RegisterManager, SpaceInfo, SpaceManager};
#[cfg(feature = "gimli")]
pub use builder::image::gimli;
pub use builder::SleighContextBuilder;

use crate::context::builder::language_def::LanguageDefinition;
use crate::context::loaded::LoadedSleighContext;
use crate::ffi::context_ffi::CTX_BUILD_MUTEX;
use crate::JingleSleighError::{ImageLoadError, SleighCompilerMutexError};
use crate::VarNode;
use cxx::{SharedPtr, UniquePtr};
use std::fmt::{Debug, Formatter};
use std::path::Path;
use crate::context::image::ImageProvider;

pub struct SleighContext {
    ctx: UniquePtr<ContextFFI>,
    spaces: Vec<SpaceInfo>,
    language_id: String,
    registers: Vec<(VarNode, String)>,
}

impl Debug for SleighContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sleigh {{arch: {}}}", self.language_id)
    }
}

impl SpaceManager for SleighContext {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.spaces.get(idx)
    }

    fn get_all_space_info(&self) -> &[SpaceInfo] {
        self.spaces.as_slice()
    }

    fn get_code_space_idx(&self) -> usize {
        self.ctx
            .getSpaceByIndex(0)
            .getManager()
            .getDefaultCodeSpace()
            .getIndex() as usize
    }
}

impl RegisterManager for SleighContext {
    fn get_register(&self, name: &str) -> Option<VarNode> {
        self.registers
            .iter()
            .find(|(_, reg_name)| reg_name.as_str() == name)
            .map(|(vn, _)| vn.clone())
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.registers
            .iter()
            .find(|(vn, _)| vn == location)
            .map(|(_, name)| name.as_str())
    }

    fn get_registers(&self) -> Vec<(VarNode, String)> {
        self.registers.clone()
    }
}

impl SleighContext {
    pub(crate) fn new<T: AsRef<Path>>(
        language_def: &LanguageDefinition,
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
                let registers = ctx
                    .getRegisters()
                    .iter()
                    .map(|b| (VarNode::from(&b.varnode), b.name.clone()))
                    .collect();

                Ok(Self {
                    ctx,
                    spaces,
                    language_id: language_def.id.clone(),
                    registers,
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
            .pin_mut()
            .set_initial_context(name, value)
            .map_err(|_| ImageLoadError)
    }

    pub fn spaces(&self) -> Vec<SharedPtr<AddrSpaceHandle>> {
        let mut spaces = Vec::with_capacity(self.ctx.getNumSpaces() as usize);
        for i in 0..self.ctx.getNumSpaces() {
            spaces.push(self.ctx.getSpaceByIndex(i))
        }
        spaces
    }

    pub fn get_language_id(&self) -> &str {
        &self.language_id
    }

    pub fn initialize_with_image<'b, T: ImageProvider + 'b>(
        self,
        img: T,
    ) -> Result<LoadedSleighContext<'b>, JingleSleighError> {
        LoadedSleighContext::new(self, img)
    }
}

#[cfg(test)]
mod test {
    use crate::context::SleighContextBuilder;
    use crate::tests::SLEIGH_ARCH;
    use crate::{RegisterManager, VarNode};

    #[test]
    fn get_regs() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        assert_ne!(sleigh.get_registers(), vec![]);
    }

    #[test]
    fn get_register_name() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        for (vn, name) in sleigh.get_registers() {
            let addr = sleigh.get_register(&name);
            assert_eq!(addr, Some(vn));
        }
    }

    #[test]
    fn get_invalid_register_name() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        assert_eq!(sleigh.get_register("fake"), None);
    }

    #[test]
    fn get_valid_register() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();

        assert_eq!(
            sleigh.get_register_name(&VarNode {
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
            sleigh.get_register_name(&VarNode {
                space_index: 40,
                offset: 5122,
                size: 1
            }),
            None
        );
    }
}
