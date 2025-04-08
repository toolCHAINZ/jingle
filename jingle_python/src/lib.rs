use ::jingle::python::instruction::PythonInstruction;
use ::jingle::python::modeled_block::PythonModeledBlock;
use ::jingle::python::modeled_instruction::PythonModeledInstruction;
use ::jingle::python::sleigh_context::LoadedSleighContextWrapper;
use ::jingle::python::sleigh_context::{create_jingle_context, create_sleigh_context};
use ::jingle::python::state::PythonState;
use ::jingle::sleigh::{IndirectVarNode, PcodeOperation, VarNode};
use pyo3::prelude::*;
use std::ffi::CString;
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
    use jingle::python::sleigh_context::create_sleigh_context;

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
