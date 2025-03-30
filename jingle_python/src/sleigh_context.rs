use crate::instruction::PythonInstruction;
use crate::jingle_context::PythonJingleContext;
use jingle::sleigh::context::image::gimli::load_with_gimli;
use jingle::sleigh::context::loaded::LoadedSleighContext;
use pyo3::{pyclass, pyfunction, pymethods, PyResult};

#[pyfunction]
pub fn create_sleigh_context(
    binary_path: &str,
    ghidra: &str,
) -> PyResult<LoadedSleighContextWrapper> {
    let context = load_with_gimli(binary_path, ghidra)?;
    Ok(LoadedSleighContextWrapper { context })
}

#[pyclass(unsendable, name = "SleighContext")]
pub struct LoadedSleighContextWrapper {
    context: LoadedSleighContext<'static>,
}

#[pymethods]
impl LoadedSleighContextWrapper {
    pub fn instruction_at(&self, offset: u64) -> Option<PythonInstruction> {
        PythonInstruction::read_from_ctx(&self.context, offset)
    }

    #[setter]
    pub fn set_base_address(&mut self, offset: u64) {
        self.context.set_base_address(offset);
    }

    #[getter]
    pub fn get_base_address(&mut self) -> u64 {
        self.context.get_base_address()
    }

    pub fn make_jingle_context(&self) -> PyResult<PythonJingleContext> {
        PythonJingleContext::make_jingle_context(&self.context)
    }
}
