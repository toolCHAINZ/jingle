use crate::state::PythonState;
use crate::varode_iterator::VarNodeIterator;
use jingle::modeling::{ModeledInstruction, ModelingContext};
use jingle::sleigh::Instruction;
use jingle::JingleContext;
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
    pub fn get_input_bvs(&self) -> VarNodeIterator {
        let filtered: Vec<_> = self
            .instr
            .get_outputs()
            .into_iter()
            .filter(|o| self.instr.should_varnode_constrain(o)).collect();
        VarNodeIterator::new(
            // intentional: that AST has the input in it too
            self.instr.get_final_state().clone(),
            filtered.into_iter(),
        )
    }


    /// A list of the output varnodes to the instruction, filtering
    /// for only those representing actual locations in processor memory:
    /// "internal" varnodes are filtered out
    pub fn get_output_bvs(&self) -> VarNodeIterator {
        let filtered: Vec<_> = self
            .instr
            .get_outputs()
            .into_iter()
            .filter(|o| self.instr.should_varnode_constrain(o)).collect();
        VarNodeIterator::new(
            // intentional: that AST has the input in it too
            self.instr.get_final_state().clone(),
            filtered.into_iter(),
        )
    }
}
