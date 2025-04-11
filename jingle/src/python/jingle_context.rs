use crate::python::z3::get_python_z3;
use crate::python::modeled_block::PythonModeledBlock;
use crate::python::modeled_instruction::PythonModeledInstruction;
use crate::JingleContext;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use jingle_sleigh::JingleSleighError::InstructionDecode;
use pyo3::prelude::*;
use std::rc::Rc;

#[pyclass(unsendable)]
pub struct PythonJingleContext {
    pub jingle: JingleContext<'static>,
    pub sleigh: Rc<LoadedSleighContext<'static>>,
}

impl PythonJingleContext {
    pub fn make_jingle_context(
        sleigh: Rc<LoadedSleighContext<'static>>,
    ) -> PyResult<PythonJingleContext> {
        let ctx = get_python_z3()?;
        let ctx = JingleContext::new(ctx, sleigh.as_ref());
        ctx.fresh_state();
        Ok(PythonJingleContext {
            jingle: ctx,
            sleigh,
        })
    }
}

#[pymethods]
impl PythonJingleContext {
    pub fn model_instruction_at(&self, offset: u64) -> PyResult<PythonModeledInstruction> {
        let instr = self
            .sleigh
            .instruction_at(offset)
            .ok_or(InstructionDecode)?;
        PythonModeledInstruction::new(instr, &self.jingle)
    }

    pub fn model_block_at(&self, offset: u64, max_instrs: usize) -> PyResult<PythonModeledBlock> {
        PythonModeledBlock::new(&self.jingle, self.sleigh.read(offset, max_instrs))
    }
}
