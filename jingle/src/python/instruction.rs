use crate::JingleContext;
use crate::display::JingleDisplayable;
use crate::sleigh::context::loaded::LoadedSleighContext;
use crate::sleigh::{ArchInfoProvider, Instruction, PcodeOperation, SpaceInfo, VarNode};
use jingle_sleigh::SleighArchInfo;
use pyo3::{pyclass, pymethods};
use std::fmt::{Display, Formatter};

#[pyclass(str, name = "Instruction")]
/// An assembly instruction parsed by SLEIGH
pub struct PythonInstruction {
    instruction: Instruction,
    info: SleighArchInfo,
}

impl PythonInstruction {
    pub fn new(instruction: &Instruction, ctx: &JingleContext) -> Self {
        Self {
            instruction: instruction.clone(),
            info: ctx.info.clone(),
        }
    }
    pub fn read_from_ctx(ctx: &LoadedSleighContext, offset: u64) -> Option<Self> {
        ctx.instruction_at(offset).map(|i| PythonInstruction {
            instruction: i,
            info: ctx.arch_info(),
        })
    }
}

impl ArchInfoProvider for &PythonInstruction {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.info.get_space_info(idx)
    }

    fn get_all_space_info(&self) -> impl Iterator<Item = &SpaceInfo> {
        self.info.get_all_space_info()
    }

    fn get_code_space_idx(&self) -> usize {
        self.info.get_code_space_idx()
    }

    fn get_register(&self, name: &str) -> Option<&VarNode> {
        self.info.get_register(name)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.info.get_register_name(location)
    }

    fn get_registers(&self) -> impl Iterator<Item = (&VarNode, &str)> {
        self.info.get_registers()
    }
}

impl Display for PythonInstruction {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let d = self.instruction.display(&self.info);
        d.fmt(f)
    }
}

#[pymethods]
impl PythonInstruction {
    #[getter]
    fn disassembly(&self) -> String {
        format!(
            "{} {}",
            self.instruction.disassembly.mnemonic, self.instruction.disassembly.args
        )
    }

    fn pcode(&self) -> Vec<PcodeOperation> {
        self.instruction.ops.clone()
    }
}
