use crate::state::PythonState;
use jingle::modeling::{ModeledInstruction, ModelingContext};
use jingle::sleigh::Instruction;
use jingle::JingleContext;
use pyo3::{pyclass, pymethods, PyResult};

#[pyclass(unsendable)]
pub struct PythonModeledInstruction {
    instr: ModeledInstruction<'static>,
}

impl PythonModeledInstruction {
    pub fn new(
        instr: Instruction,
        jingle: &JingleContext<'static>,
    ) -> PyResult<PythonModeledInstruction> {
        Ok(Self {
            instr: ModeledInstruction::new(instr, jingle)?,
        })
    }
}

#[pymethods]
impl PythonModeledInstruction {
    #[getter]
    pub fn original_state(&self) -> PythonState {
        PythonState {
            state: self.instr.get_original_state().clone(),
        }
    }

    #[getter]
    pub fn final_state(&self) -> PythonState {
        PythonState {
            state: self.instr.get_final_state().clone(),
        }
    }
}
