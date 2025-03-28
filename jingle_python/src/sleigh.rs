use jingle::sleigh::context::{SleighContext, SleighContextBuilder};
use jingle::sleigh::JingleSleighError;
use std::fmt::Debug;
use std::path::Path;
use pyo3::{pyfunction, PyResult};

fn create_sleigh_context_internal<T: AsRef<Path> + Debug>(
    t: T,
    arch: &str,
) -> Result<SleighContext, JingleSleighError> {
    SleighContextBuilder::load_ghidra_installation(t).and_then(|b| b.build(arch))
}

#[pyfunction]
pub fn create_sleigh_context(path: &str, arch: &str) -> PyResult<SleighContext> {
    let hi = create_sleigh_context_internal(path, arch)?;
    Ok(hi)
}