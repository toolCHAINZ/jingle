use crate::context_switcheroo;
use crate::modeled_block::PythonModeledBlock;
use crate::modeled_instruction::PythonModeledInstruction;
use jingle::sleigh::context::loaded::LoadedSleighContext;
use jingle::sleigh::JingleSleighError::InstructionDecode;
use jingle::JingleContext;
use pyo3::prelude::*;
use std::rc::Rc;
use z3_sys::Z3_context;

#[pyclass(unsendable)]
pub struct PythonJingleContext {
    pub jingle: JingleContext<'static>,
    pub sleigh: Rc<LoadedSleighContext<'static>>,
}

impl PythonJingleContext {
    pub fn make_jingle_context(
        sleigh: Rc<LoadedSleighContext<'static>>,
    ) -> PyResult<PythonJingleContext> {
        Python::with_gil(|py| {
            let z3_mod = PyModule::import(py, "z3")?;
            let global_ctx = z3_mod.getattr("main_ctx")?.call0()?;
            let z3_ptr: usize = global_ctx
                .getattr("ref")?
                .call0()?
                .getattr("value")?
                .extract()?;
            let raw_ctx: Z3_context = z3_ptr as Z3_context;
            let ctx = context_switcheroo(raw_ctx);
            let ctx = JingleContext::new(ctx, sleigh.as_ref());
            ctx.fresh_state();
            Ok(PythonJingleContext {
                jingle: ctx,
                sleigh,
            })
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
