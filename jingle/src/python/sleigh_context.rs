use crate::python::instruction::PythonInstruction;
use crate::python::jingle_context::PythonJingleContext;
use jingle_sleigh::context::image::gimli::load_with_gimli;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use pyo3::exceptions::PyRuntimeError;
use pyo3::{PyResult, pyclass, pyfunction, pymethods};
use std::rc::Rc;

#[pyfunction]
pub fn create_jingle_context(binary_path: &str, ghidra: &str) -> PyResult<PythonJingleContext> {
    let context = Rc::new(load_with_gimli(binary_path, ghidra)?);
    PythonJingleContext::make_jingle_context(context)
}

#[pyclass(unsendable, name = "SleighContext")]
pub struct LoadedSleighContextWrapper {
    context: Rc<LoadedSleighContext<'static>>,
}

#[pymethods]
impl LoadedSleighContextWrapper {
    #[new]
    pub fn new(binary_path: &str, ghidra: &str) -> PyResult<Self> {
        let context = Rc::new(load_with_gimli(binary_path, ghidra)?);
        Ok(LoadedSleighContextWrapper { context })
    }
    
    pub fn instruction_at(&self, offset: u64) -> Option<PythonInstruction> {
        PythonInstruction::read_from_ctx(&self.context, offset)
    }

    #[setter]
    pub fn set_base_address(&mut self, offset: u64) -> PyResult<()> {
        Rc::get_mut(&mut self.context)
            .ok_or(PyRuntimeError::new_err("sdf"))?
            .set_base_address(offset);
        Ok(())
    }

    #[getter]
    pub fn get_base_address(&mut self) -> u64 {
        self.context.get_base_address()
    }

    pub fn make_jingle_context(&self) -> PyResult<PythonJingleContext> {
        PythonJingleContext::make_jingle_context(self.context.clone())
    }
}
