use crate::modeling::{ModeledInstruction, ModelingContext};
use crate::python::state::PythonState;
use crate::python::varode_iterator::VarNodeIterator;
use crate::JingleContext;
use jingle_sleigh::Instruction;
use pyo3::{pyclass, pymethods, PyResult};

#[pyclass(unsendable)]
/// A symbolic model of a "simple" SLEIGH instruction,
/// where a "simple" instruction is one that performs only
/// INT-interpreted data transfer operations and contains no
/// nontrivial control flow
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
    /// The symbolic state before the instruction
    /// is executed
    pub fn original_state(&self) -> PythonState {
        PythonState {
            state: self.instr.get_original_state().clone(),
        }
    }

    #[getter]
    /// The symbolic state after the instruction is executed
    pub fn final_state(&self) -> PythonState {
        PythonState {
            state: self.instr.get_final_state().clone(),
        }
    }

    /// A list of the input varnodes to the instruction, filtering
    /// for only those representing actual locations in processor memory:
    /// constants and "internal" varnodes are filtered out
    pub fn get_input_bvs(&self) -> PyResult<VarNodeIterator> {
        let filtered: Result<Vec<_>,_> = self
            .instr
            .instr
            .clone()
            .ops
            .into_iter().flat_map(|op| op.inputs())
            .into_iter()
            .map(|g| g.display(self.instr.get_final_state()))
            .collect();
        let filtered = filtered?;
        Ok(VarNodeIterator::new(filtered.into_iter()))
    }

    /// A list of the output varnodes to the instruction, filtering
    /// for only those representing actual locations in processor memory:
    /// "internal" varnodes are filtered out
    pub fn get_output_bvs(&self) -> PyResult<VarNodeIterator> {
        let filtered: Result<Vec<_>,_> = self
            .instr
            .instr
            .clone()
            .ops
            .into_iter().flat_map(|op| op.output())
            .into_iter()
            .map(|g| g.display(self.instr.get_final_state()))
            .collect();
        let filtered = filtered?;
        Ok(VarNodeIterator::new(filtered.into_iter()))

    }
}
