use crate::JingleContext;
use crate::display::JingleDisplayable;
use crate::modeling::{ModeledInstruction, ModelingContext};
use crate::python::resolved_varnode::PythonResolvedVarNode;
use crate::python::state::PythonState;
use crate::python::varode_iterator::VarNodeIterator;
use jingle_sleigh::Instruction;
use pyo3::{PyResult, pyclass, pymethods};

#[pyclass(unsendable, name = "ModeledInstruction")]
/// A symbolic model of a "simple" SLEIGH instruction,
/// where a "simple" instruction is one that performs only
/// INT-interpreted data transfer operations and contains no
/// nontrivial control flow
pub struct PythonModeledInstruction {
    instr: ModeledInstruction,
}

impl PythonModeledInstruction {
    pub fn new(instr: Instruction, jingle: &JingleContext) -> PyResult<PythonModeledInstruction> {
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
        PythonState::from(self.instr.get_original_state().clone())
    }

    #[getter]
    /// The symbolic state after the instruction is executed
    pub fn final_state(&self) -> PythonState {
        PythonState::from(self.instr.get_final_state().clone())
    }

    /// A list of the input varnodes to the instruction, filtering
    /// for only those representing actual locations in processor memory:
    /// constants and "internal" varnodes are filtered out
    pub fn get_input_vns(&self) -> PyResult<VarNodeIterator> {
        let s = self.instr.get_jingle().info.clone();
        let filtered: Vec<PythonResolvedVarNode> = self
            .instr
            .get_inputs()
            .into_iter()
            .map(|g| PythonResolvedVarNode::from(g.display(&s)))
            .collect();
        let filtered = filtered.into_iter();
        Ok(VarNodeIterator::new(filtered.into_iter()))
    }

    /// A list of the output varnodes to the instruction, filtering
    /// for only those representing actual locations in processor memory:
    /// "internal" varnodes are filtered out
    pub fn get_output_vns(&self) -> PyResult<VarNodeIterator> {
        let s = self.instr.get_jingle().info.clone();
        let filtered: Vec<PythonResolvedVarNode> = self
            .instr
            .get_outputs()
            .into_iter()
            .map(|g| PythonResolvedVarNode::from(g.display(&s)))
            .collect();
        let filtered = filtered.into_iter();
        Ok(VarNodeIterator::new(filtered.into_iter()))
    }
}
