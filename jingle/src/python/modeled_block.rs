use crate::modeling::{ModeledBlock, ModelingContext};
use crate::python::state::PythonState;
use crate::python::varode_iterator::VarNodeIterator;
use crate::sleigh::Instruction;
use crate::JingleContext;
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
    pub fn get_input_bvs(&self) -> VarNodeIterator {
        let filtered: Vec<_> = self
            .instr
            .get_outputs()
            .into_iter()
            .filter(|o| self.instr.should_varnode_constrain(o))
            .collect();
        VarNodeIterator::new(
            // intentional: that AST has the input in it too
            self.instr.get_final_state().clone(),
            filtered.into_iter(),
        )
    }

    /// A list of the output varnodes to the block, filtering
    /// for only those representing actual locations in processor memory:
    /// "internal" varnodes are filtered out
    pub fn get_output_bvs(&self) -> VarNodeIterator {
        let filtered: Vec<_> = self
            .instr
            .get_outputs()
            .into_iter()
            .filter(|o| self.instr.should_varnode_constrain(o))
            .collect();
        VarNodeIterator::new(
            // intentional: that AST has the input in it too
            self.instr.get_final_state().clone(),
            filtered.into_iter(),
        )
    }
}
