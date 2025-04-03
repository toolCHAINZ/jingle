use jingle::sleigh::context::loaded::LoadedSleighContext;
use jingle::sleigh::{ArchInfoProvider, Instruction, SpaceInfo, VarNode};
use pyo3::{pyclass, pymethods};
use std::fmt::{Display, Formatter};

#[pyclass(str)]
pub struct PythonInstruction {
    instruction: Instruction,
    registers: Vec<(VarNode, String)>,
    space_names: Vec<SpaceInfo>,
    default_code_space: usize,
}

impl PythonInstruction {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let d = self.instruction.display(&self);
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
}
