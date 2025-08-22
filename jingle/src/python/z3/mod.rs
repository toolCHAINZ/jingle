use lazy_static::lazy_static;
use pyo3::types::{PyAnyMethods, PyModule};
use pyo3::{PyResult, Python};
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};
use z3::Context;
use z3_sys::Z3_context;

pub mod ast;

pub fn get_python_z3(py: Python) -> PyResult<Z3_context> {
    let z3_mod = PyModule::import(py, "z3")?;
    let global_ctx = z3_mod.getattr("main_ctx")?.call0()?;
    let z3_ptr: usize = global_ctx
        .call_method0("ref")?
        .getattr("value")?
        .extract()?;
    let raw_ctx: Z3_context = z3_ptr as Z3_context;
    Ok(raw_ctx)
}
