pub mod bitvec;
pub mod instruction;
pub mod jingle_context;
pub mod modeled_block;
pub mod modeled_instruction;
pub mod sleigh_context;
pub mod state;
pub mod varode_iterator;

use crate::instruction::PythonInstruction;
use crate::modeled_block::PythonModeledBlock;
use crate::modeled_instruction::PythonModeledInstruction;
use crate::sleigh_context::LoadedSleighContextWrapper;
use crate::state::PythonState;
use ::jingle::sleigh::{IndirectVarNode, PcodeOperation, VarNode};
use pyo3::prelude::*;
use sleigh_context::{create_jingle_context, create_sleigh_context};
use std::cell::RefCell;
use std::ffi::CString;
use std::mem;
use std::mem::ManuallyDrop;
use z3::Context;
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
pub fn context_switcheroo(z3: Z3_context) -> &'static Context {
    CONTEXT.replace(ManuallyDrop::new(Context { z3_ctx: z3 }));
    CTX_REF.with(|ctx| *ctx)
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
    m.add_class::<PythonState>()?;
    m.add_class::<PythonModeledInstruction>()?;
    m.add_class::<PythonModeledBlock>()?;
    m.add_function(wrap_pyfunction!(create_sleigh_context, m)?)?;
    m.add_function(wrap_pyfunction!(create_jingle_context, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::sleigh_context::create_sleigh_context;

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
