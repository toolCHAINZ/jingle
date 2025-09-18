use crate::python::instruction::PythonInstruction;
use jingle_sleigh::context::image::gimli::load_with_gimli;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use pyo3::exceptions::PyRuntimeError;
use pyo3::{PyResult, pyclass, pymethods};
use std::rc::Rc;
use jingle_sleigh::JingleSleighError::InstructionDecode;
use jingle_sleigh::SleighArchInfo;
use crate::python::modeled_block::PythonModeledBlock;
use crate::python::modeled_instruction::PythonModeledInstruction;

#[pyclass(unsendable, name = "SleighContext")]
pub struct PythonLoadedSleighContext {
    context: Rc<LoadedSleighContext<'static>>,
}

impl PythonLoadedSleighContext {
    pub fn arch_info(&self) -> &SleighArchInfo {
        &self.context.arch_info()
    }
}
#[pymethods]
impl PythonLoadedSleighContext {
    #[new]
    pub fn new(binary_path: &str, ghidra: &str) -> PyResult<Self> {
        let context = Rc::new(load_with_gimli(binary_path, ghidra)?);
        Ok(PythonLoadedSleighContext { context })
    }

    pub fn instruction_at(&self, offset: u64) -> Option<PythonInstruction> {
        PythonInstruction::read_from_ctx(&self.context, offset)
    }

    pub fn model_instruction_at(&self, offset: u64) -> PyResult<PythonModeledInstruction> {
        let instr = self
            .context
            .instruction_at(offset)
            .ok_or(InstructionDecode)?;
        PythonModeledInstruction::new(instr, self.context.arch_info())
    }

    pub fn model_block_at(&self, offset: u64, max_instrs: usize) -> PyResult<PythonModeledBlock> {
        PythonModeledBlock::new(self.context.arch_info(), self.context.read(offset, max_instrs))
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

}
