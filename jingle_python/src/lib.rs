mod sleigh_context;
mod instruction;

use pyo3::prelude::*;
use ::jingle::sleigh::{IndirectVarNode, PcodeOperation, VarNode};
use ::jingle::sleigh::Instruction;
use sleigh_context::create_sleigh_context;
use crate::instruction::PythonInstruction;
use crate::sleigh_context::LoadedSleighContextWrapper;


/// A Python module implemented in Rust.
#[pymodule]
fn jingle(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<VarNode>()?;
    m.add_class::<IndirectVarNode>()?;
    m.add_class::<PcodeOperation>()?;
    m.add_class::<PythonInstruction>()?;
    m.add_class::<LoadedSleighContextWrapper>()?;
    m.add_function(wrap_pyfunction!(create_sleigh_context, m)?)?;
    Ok(())
}
