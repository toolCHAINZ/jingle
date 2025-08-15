use crate::JingleContext;
use crate::display::JingleDisplayable;
use crate::modeling::{ModeledBlock, ModelingContext};
use crate::python::instruction::PythonInstruction;
use crate::python::resolved_varnode::PythonResolvedVarNode;
use crate::python::state::PythonState;
use crate::python::varode_iterator::VarNodeIterator;
use crate::sleigh::Instruction;
use pyo3::{PyResult, pyclass, pymethods};

#[pyclass(unsendable, name = "ModeledBlock")]
pub struct PythonModeledBlock {
    pub instr: ModeledBlock,
}

impl PythonModeledBlock {
    pub fn new<T: Iterator<Item = Instruction>>(
        jingle: &JingleContext,
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
    pub fn instructions(&self) -> Vec<PythonInstruction> {
        self.instr
            .instructions
            .iter()
            .map(|i| PythonInstruction::new(i, self.instr.get_jingle()))
            .collect()
    }

    #[getter]
    /// The symbolic state before the block
    /// is executed
    pub fn original_state(&self) -> PythonState {
        PythonState::from(self.instr.get_original_state().clone())
    }

    #[getter]
    /// The symbolic state after the block is executed
    pub fn final_state(&self) -> PythonState {
        PythonState::from(self.instr.get_final_state().clone())
    }

    /// A list of the input varnodes to the block, filtering
    /// for only those representing actual locations in processor memory:
    /// constants and "internal" varnodes are filtered out
    pub fn get_input_vns(&self) -> PyResult<VarNodeIterator> {
        let info = self.instr.get_jingle().info.clone();
        let filtered: Vec<PythonResolvedVarNode> = self
            .instr
            .get_inputs()
            .into_iter()
            .map(|g| PythonResolvedVarNode::from(g.display(&info)))
            .collect();
        let filtered = filtered.into_iter();
        Ok(VarNodeIterator::new(filtered))
    }

    /// A list of the output varnodes to the block, filtering
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
        Ok(VarNodeIterator::new(filtered))
    }
}
