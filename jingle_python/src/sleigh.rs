use jingle::sleigh::context::image::gimli::load_with_gimli;
use jingle::sleigh::context::loaded::LoadedSleighContext;
use jingle::sleigh::context::{SleighContext, SleighContextBuilder};
use jingle::sleigh::{Instruction, JingleSleighError};
use pyo3::{pyclass, pyfunction, pymethods, PyResult};
use std::fmt::Debug;
use std::path::Path;

fn create_sleigh_context_internal<T: AsRef<Path> + Debug>(
    t: T,
    arch: &str,
) -> Result<SleighContext, JingleSleighError> {
    SleighContextBuilder::load_ghidra_installation(t).and_then(|b| b.build(arch))
}

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
    pub fn instruction_at(&self, offset: u64) -> Option<Instruction>{
        self.context.instruction_at(offset)
    }

    #[setter]
    pub fn set_base_address(&mut self, offset: u64){
        self.context.set_base_address(offset);
    }

    #[getter]
    pub fn get_base_address(&mut self) -> u64{
        self.context.get_base_address()
    }

}