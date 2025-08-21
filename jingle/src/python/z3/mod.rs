use pyo3::types::{PyAnyMethods, PyModule};
use pyo3::{PyResult, Python};
use std::mem::ManuallyDrop;
use z3::Context;
use z3_sys::Z3_context;

pub mod ast;

pub fn get_python_z3() -> PyResult<ManuallyDrop<Context>> {
    Python::with_gil(|py| {
        let z3_mod = PyModule::import(py, "z3")?;
        let global_ctx = z3_mod.getattr("main_ctx")?.call0()?;
        let z3_ptr: usize = global_ctx
            .getattr("ref")?
            .call0()?
            .getattr("value")?
            .extract()?;
        let raw_ctx: Z3_context = z3_ptr as Z3_context;
        let ctx = unsafe { ManuallyDrop::new(Context::from_raw(raw_ctx)) };
        Ok(ctx)
    })
}
