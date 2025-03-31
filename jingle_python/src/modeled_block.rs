use crate::state::PythonState;
use jingle::modeling::{ModeledBlock, ModelingContext};
use jingle::sleigh::Instruction;
use jingle::JingleContext;
use pyo3::{pyclass, pymethods, PyResult};

#[pyclass(unsendable)]
pub struct PythonModeledBlock {
    instr: ModeledBlock<'static>,
}

impl PythonModeledBlock {
    pub fn new<T: Iterator<Item = Instruction>>(
        jingle: &JingleContext<'static>,
        i: T,
    ) -> PyResult<PythonModeledBlock> {
        Ok(Self {
            instr: ModeledBlock::read(jingle, i)?,
        })
    }
}

#[pymethods]
impl PythonModeledBlock {
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
