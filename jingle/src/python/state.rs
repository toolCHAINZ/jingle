use crate::modeling::State;
use crate::python::jingle_context::PythonJingleContext;
use crate::python::resolved_varnode::PythonResolvedVarNode;
use crate::python::z3::ast::TryIntoPythonZ3;
use crate::python::z3::get_python_z3;
use crate::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle_sleigh::{ArchInfoProvider, VarNode};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

#[pyclass(unsendable, name = "State")]
/// A symbolic p-code state
pub struct PythonState {
    state: State<'static>,
}

impl PythonState {
    pub fn state(&self) -> &State<'static> {
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
        match varnode {
            PythonResolvedVarNode::Direct(a) => self.state.read_varnode(&VarNode::from(a.clone())),
            PythonResolvedVarNode::Indirect(a) => {
                let ind = ResolvedIndirectVarNode::from(&a.inner);
                self.state.read_resolved(&ResolvedVarnode::Indirect(ind))
            }
        }?
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

impl From<State<'static>> for PythonState {
    fn from(value: State<'static>) -> Self {
        PythonState {
            state: value,
        }
    }
}
