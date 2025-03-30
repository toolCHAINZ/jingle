mod instruction;
mod jingle_context;
mod sleigh_context;
mod state;

use crate::instruction::PythonInstruction;
use crate::sleigh_context::LoadedSleighContextWrapper;
use ::jingle::sleigh::{IndirectVarNode, PcodeOperation, VarNode};
use pyo3::prelude::*;
use sleigh_context::create_sleigh_context;
use std::cell::RefCell;
use std::ffi::CString;
use std::mem;
use z3::Context;
use z3_sys::Z3_context;

thread_local! {
    pub static CONTEXT: RefCell<Z3_context> = RefCell::new(std::ptr::null_mut());
}

thread_local! {
    pub static CTX_REF: &'static Context = CONTEXT.with_borrow(|ctx| unsafe {
        mem::transmute(ctx)
    });
}
pub fn context_switcheroo(z3: Z3_context) -> &'static Context {
    CONTEXT.replace(z3);
    CTX_REF.with(|ctx| {
        println!("{:p}", *ctx);
        dbg!(*ctx)
    })
}

/// A Python module implemented in Rust.
#[pymodule]
fn jingle(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.py().run(&CString::new("import z3")?, None, None)?;
    m.add_class::<VarNode>()?;
    m.add_class::<IndirectVarNode>()?;
    m.add_class::<PcodeOperation>()?;
    m.add_class::<PythonInstruction>()?;
    m.add_class::<LoadedSleighContextWrapper>()?;
    m.add_function(wrap_pyfunction!(create_sleigh_context, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::sleigh_context::{create_sleigh_context, LoadedSleighContextWrapper};

    #[test]
    fn ctx() {
        pyo3::prepare_freethreaded_python();
        let ctx = create_sleigh_context(
            "/Users/maroed/RustroverProjects/code_reuse_synthesis_artifacts/crackers/libz.so.1",
            "/Applications/ghidra",
        )
        .unwrap();
        ctx.make_jingle_context().unwrap();
    }
}
