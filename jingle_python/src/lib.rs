use ::jingle::python::instruction::PythonInstruction;
use ::jingle::python::jingle_context::PythonJingleContext;
use ::jingle::python::modeled_block::PythonModeledBlock;
use ::jingle::python::modeled_instruction::PythonModeledInstruction;
use ::jingle::python::sleigh_context::LoadedSleighContextWrapper;
use ::jingle::python::state::PythonState;
use ::jingle::sleigh::{IndirectVarNode, PcodeOperation, VarNode};
use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
fn jingle(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<VarNode>()?;
    m.add_class::<IndirectVarNode>()?;
    m.add_class::<PcodeOperation>()?;
    m.add_class::<PythonInstruction>()?;
    m.add_class::<LoadedSleighContextWrapper>()?;
    m.add_class::<PythonJingleContext>()?;
    m.add_class::<PythonState>()?;
    m.add_class::<PythonModeledInstruction>()?;
    m.add_class::<PythonModeledBlock>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use jingle::python::sleigh_context::LoadedSleighContextWrapper;

    #[test]
    fn ctx() {
        pyo3::prepare_freethreaded_python();
        let ctx = LoadedSleighContextWrapper::new(
            "/Users/maroed/RustroverProjects/code_reuse_synthesis_artifacts/crackers/libz.so.1",
            "/Applications/ghidra",
        )
        .unwrap();
        ctx.make_jingle_context().unwrap();
    }
}
