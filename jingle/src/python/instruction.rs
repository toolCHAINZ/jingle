use crate::display::JingleDisplayable;
use crate::sleigh::context::loaded::LoadedSleighContext;
use crate::sleigh::{Instruction, PcodeOperation};
use jingle_sleigh::SleighArchInfo;
use pyo3::{pyclass, pymethods};
use std::borrow::Borrow;
use std::fmt::{Display, Formatter};

#[pyclass(str, name = "Instruction")]
/// An assembly instruction parsed by SLEIGH
pub struct PythonInstruction {
    instruction: Instruction,
    info: SleighArchInfo,
}

impl PythonInstruction {
    pub fn new<T: Borrow<SleighArchInfo>>(instruction: &Instruction, ctx: T) -> Self {
        Self {
            instruction: instruction.clone(),
            info: ctx.borrow().clone(),
        }
    }
    pub fn read_from_ctx(ctx: &LoadedSleighContext, offset: u64) -> Option<Self> {
        ctx.instruction_at(offset).map(|i| PythonInstruction {
            instruction: i,
            info: ctx.arch_info().clone(),
        })
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
