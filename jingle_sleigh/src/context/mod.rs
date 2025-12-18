mod builder;
pub mod image;
mod instruction_iterator;
pub mod loaded;
mod python;

use crate::error::JingleSleighError;
use crate::error::JingleSleighError::{LanguageSpecRead, SleighInitError};
use crate::ffi::addrspace::bridge::AddrSpaceHandle;
use crate::ffi::context_ffi::bridge::ContextFFI;
use crate::space::{SleighArchInfo, SleighArchInfoInner, SpaceInfo};
pub use builder::SleighContextBuilder;
use std::collections::HashMap;

use crate::JingleSleighError::{ImageLoadError, SleighCompilerMutexError};
use crate::VarNode;
use crate::context::builder::language_def::Language;
use crate::context::image::ImageProvider;
use crate::context::loaded::LoadedSleighContext;
use crate::ffi::context_ffi::CTX_BUILD_MUTEX;
use cxx::{SharedPtr, UniquePtr};
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::sync::Arc;

pub struct SleighContext {
    ctx: UniquePtr<ContextFFI>,
    language_id: String,
    arch_info: SleighArchInfo,
}

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
                    }),
                };

                Ok(Self {
                    ctx,
                    arch_info,
                    language_id: language_def.id.clone(),
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

    pub fn arch_info(&self) -> &SleighArchInfo {
        &self.arch_info
    }

    pub fn initialize_with_image<'b, T: ImageProvider + 'b>(
        self,
        img: T,
    ) -> Result<LoadedSleighContext<'b>, JingleSleighError> {
        LoadedSleighContext::new(self, img)
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
        assert_eq!(instr.ops.len(), 3);
        // the stages of a push in pcode
        assert_eq!(instr.ops[0].opcode(), OpCode::CPUI_COPY);
        assert_eq!(instr.ops[1].opcode(), OpCode::CPUI_INT_SUB);
        assert_eq!(instr.ops[2].opcode(), OpCode::CPUI_STORE);
    }
}
