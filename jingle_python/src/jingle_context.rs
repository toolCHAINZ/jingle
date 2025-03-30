use crate::context_switcheroo;
use jingle::sleigh::ArchInfoProvider;
use jingle::JingleContext;
use pyo3::prelude::*;
use z3_sys::Z3_context;

#[pyclass(unsendable)]
pub struct PythonJingleContext {
    #[allow(unused)]
    context: JingleContext<'static>,
}

impl PythonJingleContext {
    pub fn make_jingle_context<T: ArchInfoProvider>(i: &T) -> PyResult<PythonJingleContext> {
        Python::with_gil(|py| {
            let z3_mod = PyModule::import(py, "z3")?;
            let global_ctx = z3_mod.getattr("main_ctx")?.call0()?;
            let z3_ptr: usize = global_ctx
                .getattr("ref")?
                .call0()?
                .getattr("value")?
                .extract()?;
            println!("z3_ptr: {:x}", z3_ptr);
            let raw_ctx: Z3_context = z3_ptr as Z3_context;
            let ctx = context_switcheroo( raw_ctx );
            let ctx = JingleContext::new(ctx, i);
            ctx.fresh_state();
            Ok(PythonJingleContext { context: ctx })
        })
    }
}
