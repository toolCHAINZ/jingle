use crate::sleigh::context::loaded::LoadedSleighContext;
use crate::sleigh::{ArchInfoProvider, Instruction, PcodeOperation, SpaceInfo, VarNode};
use pyo3::{pyclass, pymethods};
use std::fmt::{Display, Formatter};
use crate::display::JingleDisplayable;
use crate::JingleContext;

#[pyclass(str, name = "Instruction")]
/// An assembly instruction parsed by SLEIGH
pub struct PythonInstruction {
    instruction: Instruction,
    jingle: JingleContext,
}

impl PythonInstruction {
    pub fn new<T: ArchInfoProvider>(instruction: &Instruction, ctx: &T) -> Self {
        Self {
            instruction: instruction.clone(),
            space_names: ctx.get_all_space_info().cloned().collect(),
            registers: ctx
                .get_registers()
                .map(|(a, b)| (a.clone(), b.to_string()))
                .collect(),
            default_code_space: ctx.get_code_space_idx(),
        }
    }
    pub fn read_from_ctx(ctx: &LoadedSleighContext, offset: u64) -> Option<Self> {
        ctx.instruction_at(offset).map(|i| PythonInstruction {
            instruction: i,
            space_names: ctx.get_all_space_info().cloned().collect(),
            registers: ctx
                .get_registers()
                .map(|(a, b)| (a.clone(), b.to_string()))
                .collect(),
            default_code_space: ctx.get_code_space_idx(),
        })
    }
}

impl ArchInfoProvider for &PythonInstruction {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.space_names.get(idx)
    }

    fn get_all_space_info(&self) -> impl Iterator<Item = &SpaceInfo> {
        self.space_names.iter()
    }

    fn get_code_space_idx(&self) -> usize {
        self.default_code_space
    }

    fn get_register(&self, name: &str) -> Option<&VarNode> {
        self.registers.iter().find(|a| a.1 == name).map(|a| &a.0)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.registers
            .iter()
            .find(|a| a.0 == *location)
            .map(|a| a.1.as_str())
    }

    fn get_registers(&self) -> impl Iterator<Item = (&VarNode, &str)> {
        self.registers.iter().map(|(a, b)| (a, b.as_str()))
    }
}

impl Display for PythonInstruction {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let d = self.instruction.display(&self.);
        d?.fmt(f)
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
