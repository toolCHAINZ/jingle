mod sleigh;

use pyo3::prelude::*;
use ::jingle::sleigh::{IndirectVarNode, PcodeOperation, VarNode};
use ::jingle::sleigh::Instruction;
use sleigh::create_sleigh_context;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

/// A Python module implemented in Rust.
#[pymodule]
fn jingle(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<VarNode>()?;
    m.add_class::<IndirectVarNode>()?;
    m.add_class::<PcodeOperation>()?;
    m.add_class::<Instruction>()?;
    m.add_function(wrap_pyfunction!(create_sleigh_context, m)?)?;
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    Ok(())
}
