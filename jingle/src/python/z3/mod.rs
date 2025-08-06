use pyo3::types::{PyAnyMethods, PyModule};
use pyo3::{PyResult, Python};
use std::cell::RefCell;
use std::mem;
use std::mem::{ManuallyDrop, MaybeUninit};
use z3::Context;
use z3_sys::Z3_context;

pub mod ast;
pub mod bitvec;
pub mod bool;

thread_local! {
    pub static CONTEXT: RefCell<MaybeUninit<ManuallyDrop<Context>>> = const {
        RefCell::new(MaybeUninit::zeroed())
    };
    pub static CONTEXT_INITED: RefCell<bool> = const {RefCell::new(false)};
}

thread_local! {
    pub static CTX_REF: &'static ManuallyDrop<Context> = CONTEXT.with_borrow(|ctx| unsafe {
        mem::transmute(ctx.assume_init_ref())
    });
}
fn context_switcheroo(z3: Z3_context) -> &'static ManuallyDrop<Context> {
    if !CONTEXT_INITED.with(|r| *r.borrow()) {
        CONTEXT.replace(MaybeUninit::new(ManuallyDrop::new(unsafe {
            Context::from_raw(z3)
        })));
        CONTEXT_INITED.replace(true);
    }
    CTX_REF.with(|ctx| *(ctx))
}

pub fn get_python_z3() -> PyResult<&'static ManuallyDrop<Context>> {
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
