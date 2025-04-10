use pyo3::prelude::PyAnyMethods;
use pyo3::types::PyModule;
use pyo3::{IntoPyObject, IntoPyObjectExt, Py, PyAny, PyResult, Python};
use std::cell::RefCell;
use std::mem;
use std::mem::ManuallyDrop;
use z3::Context;
use z3::ast::{Ast, BV};
use z3_sys::Z3_context;

thread_local! {
    pub static CONTEXT: RefCell<ManuallyDrop<Context>> = const {
        RefCell::new(ManuallyDrop::new(Context{z3_ctx: std::ptr::null_mut()}))
    };
}

thread_local! {
    pub static CTX_REF: &'static Context = CONTEXT.with_borrow(|ctx| unsafe {
        mem::transmute(ctx)
    });
}
fn context_switcheroo(z3: Z3_context) -> &'static Context {
    CONTEXT.replace(ManuallyDrop::new(Context { z3_ctx: z3 }));
    CTX_REF.with(|ctx| *ctx)
}

pub fn get_python_z3() -> PyResult<&'static Context> {
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
        Ok(ctx)
    })
}

pub fn adapt_bv(bv: BV) -> PyResult<Py<PyAny>> {
    Python::with_gil(|py: Python| {
        let z3_mod = PyModule::import(py, "z3")?;
        let ref_class = z3_mod.getattr("BitVecRef")?.into_pyobject(py)?;
        let ctypes = PyModule::import(py, "ctypes")?;
        let ptr_type = ctypes.getattr("c_void_p")?;
        let args = bv.get_z3_ast() as usize;
        let ptr = ptr_type.call1((args,))?;

        let a = ref_class.call1((ptr,))?.into_py_any(py)?;
        Ok(a)
    })
}
