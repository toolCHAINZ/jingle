use crate::state::PythonState;
use crate::varode_iterator::VarNodeIterator;
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

    pub fn get_input_bvs(&self) -> VarNodeIterator {
        VarNodeIterator::new(
            // intentional: that AST has the input in it too
            self.instr.get_final_state().clone(),
            self.instr.get_inputs().clone().into_iter(),
        )
    }

    pub fn get_output_bvs(&self) -> VarNodeIterator {
        VarNodeIterator::new(
            // intentional: that AST has the input in it too
            self.instr.get_final_state().clone(),
            self.instr.get_outputs().clone().into_iter(),
        )
    }
}
