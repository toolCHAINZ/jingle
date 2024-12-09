mod builder;
pub mod image;
mod instruction_iterator;
pub mod loaded;
mod symbol_display;

use crate::error::JingleSleighError;
use crate::error::JingleSleighError::{LanguageSpecRead, SleighInitError};
use crate::ffi::addrspace::bridge::AddrSpaceHandle;
use crate::ffi::context_ffi::bridge::{ContextFFI, InstructionFFI};
use crate::space::{RegisterManager, SpaceInfo, SpaceManager};
pub use builder::SleighContextBuilder;

use crate::context::builder::language_def::LanguageDefinition;
use crate::context::image::ImageProvider;
use crate::context::loaded::LoadedSleighContext;
use crate::context::symbol_display::SymbolizedPcodeOperationDisplay;
use crate::ffi::context_ffi::CTX_BUILD_MUTEX;
use crate::ffi::instruction::bridge::{RawPcodeOp, VarnodeInfoFFI};
use crate::pcode::PcodeOperation::*;
use crate::varnode::display::symbolized::{
    SymbolizedIndirectVarNodeDisplay, SymbolizedVarNodeDisplay,
};
use crate::JingleSleighError::{ImageLoadError, InvalidSpaceName, SleighCompilerMutexError};
use crate::{IndirectVarNode, Instruction, OpCode, PcodeOperation, SharedSpaceInfo, VarNode};
use cxx::{SharedPtr, UniquePtr};
use std::fmt::{Debug, Formatter};
use std::path::Path;
use std::rc::Rc;

pub struct SleighContext {
    ctx: UniquePtr<ContextFFI>,
    spaces: Vec<SharedSpaceInfo>,
    language_id: String,
    registers: Vec<(VarNode, String)>,
}

impl Debug for SleighContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sleigh {{arch: {}}}", self.language_id)
    }
}

impl SpaceManager for SleighContext {
    fn get_space_info(&self, idx: usize) -> Option<&SharedSpaceInfo> {
        self.spaces.get(idx)
    }

    fn get_all_space_info(&self) -> impl Iterator<Item = &SharedSpaceInfo> {
        self.spaces.iter()
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
    pub fn varnode(
        &self,
        space_name: &str,
        offset: u64,
        size: usize,
    ) -> Result<VarNode, JingleSleighError> {
        let space = self
            .spaces
            .iter()
            .find(|s| s.name == space_name)
            .ok_or(InvalidSpaceName)?
            .clone();
        Ok(VarNode {
            space,
            offset,
            size,
        })
    }
    fn translate_varnode(&self, vn_ffi: &VarnodeInfoFFI) -> VarNode {
        VarNode {
            space: self.spaces[vn_ffi.space.getIndex() as usize].clone(),
            offset: vn_ffi.offset,
            size: vn_ffi.size,
        }
    }

    fn apply_symbols_to_varnode(&self, op: &VarNode) -> SymbolizedVarNodeDisplay {
        if let Some(s) = self.get_register_name(op) {
            SymbolizedVarNodeDisplay::Symbol(s.to_string())
        } else {
            SymbolizedVarNodeDisplay::VarNode(op.clone())
        }
    }

    fn apply_symbols_to_indirect_varnode(
        &self,
        op: &IndirectVarNode,
    ) -> SymbolizedIndirectVarNodeDisplay {
        SymbolizedIndirectVarNodeDisplay {
            pointer_space: op.pointer_space.clone(),
            access_size_bytes: op.access_size_bytes,
            pointer_location: self.apply_symbols_to_varnode(&op.pointer_location),
        }
    }

    pub fn apply_symbols_to_operation<'a, 'b>(
        &'a self,
        op: &'b PcodeOperation,
    ) -> SymbolizedPcodeOperationDisplay<'a, 'b> {
        SymbolizedPcodeOperationDisplay {
            operation: op,
            sleigh: self,
        }
    }
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
                let mut spaces: Vec<SharedSpaceInfo> =
                    Vec::with_capacity(ctx.getNumSpaces() as usize);
                for idx in 0..ctx.getNumSpaces() {
                    spaces.push(Rc::new(SpaceInfo::from(ctx.getSpaceByIndex(idx))).into());
                }
                let mut s = Self {
                    ctx,
                    spaces,
                    language_id: language_def.id.clone(),
                    registers: Vec::new(),
                };
                let registers = s
                    .ctx
                    .getRegisters()
                    .iter()
                    .map(|b| (s.translate_varnode(&b.varnode), b.name.clone()))
                    .collect();
                s.registers = registers;
                Ok(s)
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

    pub fn translate_operation(&self, value: RawPcodeOp) -> PcodeOperation {
        macro_rules! one_in {
            ($op:tt) => {
                $op {
                    input: self.translate_varnode(&value.inputs[0]),
                }
            };
        }

        macro_rules! one_in_indirect {
            ($op:tt) => {
                $op {
                    input: IndirectVarNode {
                        pointer_location: self.translate_varnode(&value.inputs[0]),
                        access_size_bytes: value.space.getAddrSize() as usize,
                        pointer_space: self.spaces[value.space.getIndex() as usize].clone().into(),
                    },
                }
            };
        }

        macro_rules! two_in {
            ($op:tt) => {
                $op {
                    input0: self.translate_varnode(&value.inputs[0]),
                    input1: self.translate_varnode(&value.inputs[1]),
                }
            };
        }
        macro_rules! one_in_one_out {
            ($op:tt) => {
                $op {
                    output: self.translate_varnode(&value.output),
                    input: self.translate_varnode(&value.inputs[0]),
                }
            };
        }
        macro_rules! two_in_one_out {
            ($op:tt) => {
                $op {
                    output: self.translate_varnode(&value.output),
                    input0: self.translate_varnode(&value.inputs[0]),
                    input1: self.translate_varnode(&value.inputs[1]),
                }
            };
        }
        match value.op {
            OpCode::CPUI_COPY => one_in_one_out!(Copy),
            OpCode::CPUI_LOAD => {
                let space_id = value.inputs[0].offset;
                let space = value.inputs[0]
                    .space
                    .getManager()
                    .getSpaceFromPointer(space_id);
                let output = self.translate_varnode(&value.output);
                Load {
                    input: IndirectVarNode {
                        pointer_space: self.spaces[space.getIndex() as usize].clone(),
                        pointer_location: self.translate_varnode(&value.inputs[1]),
                        access_size_bytes: output.size,
                    },
                    output: self.translate_varnode(&value.output),
                }
            }
            OpCode::CPUI_STORE => {
                let space_id = value.inputs[0].offset;
                let space = value.inputs[0]
                    .space
                    .getManager()
                    .getSpaceFromPointer(space_id);
                let input = self.translate_varnode(&value.inputs[2]);
                Store {
                    output: IndirectVarNode {
                        pointer_space: self.spaces[space.getIndex() as usize].clone(),
                        pointer_location: self.translate_varnode(&value.inputs[1]),
                        access_size_bytes: input.size,
                    },
                    input,
                }
            }
            OpCode::CPUI_BRANCH => one_in!(Branch),
            OpCode::CPUI_CBRANCH => two_in!(CBranch),
            OpCode::CPUI_BRANCHIND => one_in_indirect!(BranchInd),
            OpCode::CPUI_CALL => one_in!(Call),
            OpCode::CPUI_CALLIND => one_in_indirect!(CallInd),
            OpCode::CPUI_CALLOTHER => {
                let output = match value.has_output {
                    true => Some(self.translate_varnode(&value.output)),
                    false => None,
                };
                //let inputs: Vec<VarNode> = Vec::with_capacity(value.inputs.len());
                let inputs: Vec<VarNode> = value
                    .inputs
                    .iter()
                    .map(|i| self.translate_varnode(i))
                    .collect();
                CallOther { inputs, output }
            }
            OpCode::CPUI_RETURN => one_in_indirect!(Return),
            OpCode::CPUI_INT_EQUAL => two_in_one_out!(IntEqual),
            OpCode::CPUI_INT_NOTEQUAL => two_in_one_out!(IntNotEqual),
            OpCode::CPUI_INT_SLESS => two_in_one_out!(IntSignedLess),
            OpCode::CPUI_INT_SLESSEQUAL => two_in_one_out!(IntSignedLessEqual),
            OpCode::CPUI_INT_LESS => two_in_one_out!(IntLess),
            OpCode::CPUI_INT_LESSEQUAL => two_in_one_out!(IntLessEqual),
            OpCode::CPUI_INT_ZEXT => one_in_one_out!(IntZExt),
            OpCode::CPUI_INT_SEXT => one_in_one_out!(IntSExt),
            OpCode::CPUI_INT_ADD => two_in_one_out!(IntAdd),
            OpCode::CPUI_INT_SUB => two_in_one_out!(IntSub),
            OpCode::CPUI_INT_CARRY => two_in_one_out!(IntCarry),
            OpCode::CPUI_INT_SCARRY => two_in_one_out!(IntSignedCarry),
            OpCode::CPUI_INT_SBORROW => two_in_one_out!(IntSignedBorrow),
            OpCode::CPUI_INT_2COMP => one_in_one_out!(Int2Comp),
            OpCode::CPUI_INT_NEGATE => one_in_one_out!(IntNegate),
            OpCode::CPUI_INT_XOR => two_in_one_out!(IntXor),
            OpCode::CPUI_INT_AND => two_in_one_out!(IntAnd),
            OpCode::CPUI_INT_OR => two_in_one_out!(IntOr),
            OpCode::CPUI_INT_LEFT => two_in_one_out!(IntLeftShift),
            OpCode::CPUI_INT_RIGHT => two_in_one_out!(IntRightShift),
            OpCode::CPUI_INT_SRIGHT => two_in_one_out!(IntSignedRightShift),
            OpCode::CPUI_INT_MULT => two_in_one_out!(IntMult),
            OpCode::CPUI_INT_DIV => two_in_one_out!(IntDiv),
            OpCode::CPUI_INT_SDIV => two_in_one_out!(IntSignedDiv),
            OpCode::CPUI_INT_REM => two_in_one_out!(IntRem),
            OpCode::CPUI_INT_SREM => two_in_one_out!(IntSignedRem),
            OpCode::CPUI_BOOL_NEGATE => one_in_one_out!(BoolNegate),
            OpCode::CPUI_BOOL_XOR => two_in_one_out!(BoolXor),
            OpCode::CPUI_BOOL_AND => two_in_one_out!(BoolAnd),
            OpCode::CPUI_BOOL_OR => two_in_one_out!(BoolOr),
            OpCode::CPUI_FLOAT_EQUAL => two_in_one_out!(FloatEqual),
            OpCode::CPUI_FLOAT_NOTEQUAL => two_in_one_out!(FloatNotEqual),
            OpCode::CPUI_FLOAT_LESS => two_in_one_out!(FloatLess),
            OpCode::CPUI_FLOAT_LESSEQUAL => two_in_one_out!(FloatLessEqual),
            OpCode::CPUI_FLOAT_NAN => one_in_one_out!(FloatNaN),
            OpCode::CPUI_FLOAT_ADD => two_in_one_out!(FloatAdd),
            OpCode::CPUI_FLOAT_DIV => two_in_one_out!(FloatDiv),
            OpCode::CPUI_FLOAT_MULT => two_in_one_out!(FloatMult),
            OpCode::CPUI_FLOAT_SUB => two_in_one_out!(FloatSub),
            OpCode::CPUI_FLOAT_NEG => one_in_one_out!(FloatNeg),
            OpCode::CPUI_FLOAT_ABS => one_in_one_out!(FloatAbs),
            OpCode::CPUI_FLOAT_SQRT => one_in_one_out!(FloatSqrt),
            OpCode::CPUI_FLOAT_INT2FLOAT => one_in_one_out!(FloatIntToFloat),
            OpCode::CPUI_FLOAT_FLOAT2FLOAT => one_in_one_out!(FloatFloatToFloat),
            OpCode::CPUI_FLOAT_TRUNC => one_in_one_out!(FloatTrunc),
            OpCode::CPUI_FLOAT_CEIL => one_in_one_out!(FloatCeil),
            OpCode::CPUI_FLOAT_FLOOR => one_in_one_out!(FloatFloor),
            OpCode::CPUI_FLOAT_ROUND => one_in_one_out!(FloatRound),
            OpCode::CPUI_MULTIEQUAL => MultiEqual {
                output: self.translate_varnode(&value.output),
                input0: self.translate_varnode(&value.inputs[0]),
                input1: self.translate_varnode(&value.inputs[1]),
                // todo: actually parse out extra args. This never happens in raw pcode so punting for now.
                inputs: Vec::new(),
            },
            OpCode::CPUI_INDIRECT => two_in_one_out!(Indirect),
            OpCode::CPUI_PIECE => two_in_one_out!(Piece),
            OpCode::CPUI_SUBPIECE => two_in_one_out!(SubPiece),
            OpCode::CPUI_CAST => one_in_one_out!(Cast),
            OpCode::CPUI_PTRADD => PtrAdd {
                output: self.translate_varnode(&value.output),
                input0: self.translate_varnode(&value.inputs[0]),
                input1: self.translate_varnode(&value.inputs[1]),
                input2: self.translate_varnode(&value.inputs[2]),
            },
            OpCode::CPUI_PTRSUB => PtrSub {
                output: self.translate_varnode(&value.output),
                input0: self.translate_varnode(&value.inputs[0]),
                input1: self.translate_varnode(&value.inputs[1]),
            },
            OpCode::CPUI_SEGMENTOP => SegmentOp {
                output: self.translate_varnode(&value.output),
                //todo: based on ghidra source, we likely want to extract some other piece
                // of info here from the FFI object for input0's address space instead of
                // storing the varnode
                input0: self.translate_varnode(&value.inputs[0]),
                input1: self.translate_varnode(&value.inputs[1]),
                input2: self.translate_varnode(&value.inputs[2]),
            },
            OpCode::CPUI_CPOOLREF => CPoolRef {
                output: self.translate_varnode(&value.output),
                input0: self.translate_varnode(&value.inputs[0]),
                input1: self.translate_varnode(&value.inputs[1]),
                // todo: actually parse out extra args. This never happens in raw pcode so punting for now.
                inputs: Vec::new(),
            },
            OpCode::CPUI_NEW => New {
                output: self.translate_varnode(&value.output),
                input: self.translate_varnode(&value.inputs[0]),
                size: value.inputs.get(1).map(|v| self.translate_varnode(v)),
            },
            OpCode::CPUI_INSERT => Insert {
                output: self.translate_varnode(&value.output),
                input0: self.translate_varnode(&value.inputs[0]),
                input1: self.translate_varnode(&value.inputs[1]),
                position: self.translate_varnode(&value.inputs[2]),
                size: self.translate_varnode(&value.inputs[3]),
            },
            OpCode::CPUI_EXTRACT => Extract {
                output: self.translate_varnode(&value.output),
                input0: self.translate_varnode(&value.inputs[0]),
                position: self.translate_varnode(&value.inputs[1]),
                size: self.translate_varnode(&value.inputs[2]),
            },
            OpCode::CPUI_POPCOUNT => one_in_one_out!(PopCount),
            OpCode::CPUI_LZCOUNT => one_in_one_out!(LzCount),
            // Sleigh should not be emitting any other values.
            _ => unreachable!(),
        }
    }

    pub fn translate_instruction(&self, i: InstructionFFI) -> Instruction {
        Instruction {
            length: i.length,
            disassembly: i.disassembly,
            address: i.address,
            ops: i
                .ops
                .into_iter()
                .map(|i| self.translate_operation(i))
                .collect(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::context::SleighContextBuilder;
    use crate::tests::SLEIGH_ARCH;
    use crate::RegisterManager;

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
            sleigh.get_register_name(&sleigh.varnode("register", 512, 1).unwrap()),
            Some("CF")
        );
    }

    #[test]
    fn get_invalid_register() {
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();

        assert_eq!(
            sleigh.get_register_name(&sleigh.varnode("ram", 4, 4).unwrap()),
            None
        );
    }
}
