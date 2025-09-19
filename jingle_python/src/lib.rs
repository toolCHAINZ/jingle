use ::jingle::python::instruction::PythonInstruction;
use ::jingle::python::modeled_block::PythonModeledBlock;
use ::jingle::python::modeled_instruction::PythonModeledInstruction;
use ::jingle::python::resolved_varnode::PythonResolvedVarNode;
use ::jingle::python::sleigh_context::PythonLoadedSleighContext;
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
    m.add_class::<PythonLoadedSleighContext>()?;
    m.add_class::<PythonState>()?;
    m.add_class::<PythonModeledInstruction>()?;
    m.add_class::<PythonResolvedVarNode>()?;
    m.add_class::<PythonModeledBlock>()?;
    Ok(())
}
