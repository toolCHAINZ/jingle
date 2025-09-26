use pyo3::exceptions::PyRuntimeError;
use pyo3::types::{PyAnyMethods, PyModule};
use pyo3::{PyResult, Python};
use std::ptr::NonNull;
use z3_sys::{_Z3_context, Z3_context};

pub mod ast;

pub fn get_python_z3(py: Python) -> PyResult<Z3_context> {
    let z3_mod = PyModule::import(py, "z3")?;
    let global_ctx = z3_mod.getattr("main_ctx")?.call0()?;
    let z3_ptr: usize = global_ctx
        .call_method0("ref")?
        .getattr("value")?
        .extract()?;
    let raw_ctx: Z3_context = NonNull::new(z3_ptr as *mut _Z3_context)
        .ok_or(PyRuntimeError::new_err("Failed to get Z3 context"))?;
    Ok(raw_ctx)
}
