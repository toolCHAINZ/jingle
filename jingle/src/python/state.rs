use crate::modeling::State;
use crate::python::jingle_context::PythonJingleContext;
use crate::python::resolved_varnode::{PythonResolvedVarNode, PythonResolvedVarNodeInner};
use crate::python::z3::ast::{PythonAst, TryIntoPythonZ3};
use crate::python::z3::get_python_z3;
use crate::varnode::ResolvedVarnode;
use jingle_sleigh::{ArchInfoProvider, VarNode};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use z3::Translate;

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
    pub fn new(j: PyRef<PythonJingleContext>) -> PyResult<PythonState> {
        Ok(PythonState {
            state: State::new(&j.jingle),
        })
    }

    /// Read a varnode from the symbolic state
    pub fn varnode(&self, varnode: &PythonResolvedVarNode) -> PyResult<Py<PyAny>> {
        self.state.read_resolved(varnode.inner.inner())?
        .try_into_python()
    }

    /// Convenience function to read a named register from the symbolic state
    pub fn register(&self, name: &str) -> PyResult<Py<PyAny>> {
        let vn = self
            .state
            .get_register(name)
            .ok_or(PyRuntimeError::new_err("Queried nonexistent register"))?;
        self.state.read_varnode(vn)?.try_into_python()
    }

    /// Convenience function to read a slice from the symbolic  state of the default "code space"
    pub fn ram(&self, offset: u64, length: usize) -> PyResult<Py<PyAny>> {
        self.state
            .read_varnode(&VarNode {
                offset,
                size: length,
                space_index: self.state.get_code_space_idx(),
            })?
            .try_into_python()
    }
}

impl From<State> for PythonState {
    fn from(value: State) -> Self {
        Python::with_gil(|_| {
            let z3_py = get_python_z3().unwrap();
            PythonState {
                state: value.translate(&z3_py),
            }
        })
    }
}
