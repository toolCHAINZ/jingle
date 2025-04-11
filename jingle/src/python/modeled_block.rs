use crate::modeling::{ModeledBlock, ModelingContext};
use crate::python::state::PythonState;
use crate::python::varode_iterator::VarNodeIterator;
use crate::sleigh::Instruction;
use crate::JingleContext;
use jingle_sleigh::GeneralizedVarNodeDisplay;
use pyo3::{pyclass, pymethods, PyResult};

#[pyclass(unsendable)]
pub struct PythonModeledBlock {
    pub instr: ModeledBlock<'static>,
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
    /// The symbolic state before the block
    /// is executed
    pub fn original_state(&self) -> PythonState {
        PythonState {
            state: self.instr.get_original_state().clone(),
        }
    }

    #[getter]
    /// The symbolic state after the block is executed
    pub fn final_state(&self) -> PythonState {
        PythonState {
            state: self.instr.get_final_state().clone(),
        }
    }

    /// A list of the input varnodes to the block, filtering
    /// for only those representing actual locations in processor memory:
    /// constants and "internal" varnodes are filtered out
    pub fn get_input_bvs(&self) -> PyResult<VarNodeIterator> {
        let filtered: Result<Vec<GeneralizedVarNodeDisplay>, _> = self
            .instr
            .instructions
            .clone()
            .into_iter()
            .flat_map(|i| i.ops)
            .flat_map(|op| op.inputs())
            .into_iter()
            .map(|g| g.display(self.instr.get_final_state()))
            .collect();
        let filtered = filtered?;
        Ok(VarNodeIterator::new(filtered.into_iter()))
    }

    /// A list of the output varnodes to the block, filtering
    /// for only those representing actual locations in processor memory:
    /// "internal" varnodes are filtered out
    pub fn get_output_bvs(&self) -> PyResult<VarNodeIterator> {
        let filtered: Result<Vec<GeneralizedVarNodeDisplay>, _> = self
            .instr
            .instructions
            .clone()
            .into_iter()
            .flat_map(|i| i.ops)
            .flat_map(|op| op.output())
            .into_iter()
            .map(|g| g.display(self.instr.get_final_state()))
            .collect();
        let filtered = filtered?;
        Ok(VarNodeIterator::new(filtered.into_iter()))
    }
}
