use crate::modeling::State;
use crate::python::resolved_varnode::PythonResolvedVarNode;
use crate::python::z3::ast::PythonAst;
use jingle_sleigh::VarNode;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use crate::python::sleigh_context::PythonLoadedSleighContext;

#[pyclass(unsendable, name = "State")]
/// A symbolic p-code state
pub struct PythonState {
    state: State,
}

impl PythonState {
    pub fn state(&self) -> &State {
        &self.state
    }
}

#[pymethods]
impl PythonState {
    #[new]
    /// Creates a "fresh" state for a given sleigh configuration
    pub fn new(j: PyRef<PythonLoadedSleighContext>) -> PyResult<PythonState> {
        Ok(PythonState {
            state: State::new(j.arch_info()),
        })
    }

    /// Read a varnode from the symbolic state
    pub fn varnode(&self, varnode: &PythonResolvedVarNode) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            self.state
                .read_resolved(varnode.inner.inner())?
                .try_into_python(py)
        })
    }

    /// Convenience function to read a named register from the symbolic state
    pub fn register(&self, name: &str) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            let vn = self
                .state.arch_info()
                .register(name)
                .ok_or(PyRuntimeError::new_err("Queried nonexistent register"))?;
            self.state.read_varnode(vn)?.try_into_python(py)
        })
    }

    /// Convenience function to read a slice from the symbolic  state of the default "code space"
    pub fn ram(&self, offset: u64, length: usize) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            self.state
                .read_varnode(&VarNode {
                    offset,
                    size: length,
                    space_index: self.state.get_default_code_space_info().index,
                })?
                .try_into_python(py)
        })
    }
}

impl From<State> for PythonState {
    fn from(value: State) -> Self {
        PythonState { state: value }
    }
}
