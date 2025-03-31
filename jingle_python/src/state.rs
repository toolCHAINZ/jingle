use crate::bitvec::adapt_bv;
use crate::jingle_context::PythonJingleContext;
use jingle::modeling::State;
use jingle::sleigh::{ArchInfoProvider, VarNode};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

#[pyclass(unsendable, name = "State")]
pub struct PythonState {
    state: State<'static>,
}

#[pymethods]
impl PythonState {
    #[new]
    pub fn new(j: PyRef<PythonJingleContext>) -> PyResult<PythonState> {
        Ok(PythonState {
            state: State::new(&j.jingle),
        })
    }

    pub fn varnode(&self, varnode: &VarNode) -> PyResult<Py<PyAny>> {
        adapt_bv(self.state.read_varnode(varnode)?)
    }

    pub fn register(&self, name: &str) -> PyResult<Py<PyAny>> {
        let vn = self
            .state
            .get_register(name)
            .ok_or(PyRuntimeError::new_err("Queried nonexistent register"))?;
        adapt_bv(self.state.read_varnode(vn)?)
    }

    pub fn ram(&self, offset: u64, length: usize) -> PyResult<Py<PyAny>> {
        adapt_bv(self.state.read_varnode(&VarNode {
            offset,
            size: length,
            space_index: self.state.get_code_space_idx(),
        })?)
    }
}
