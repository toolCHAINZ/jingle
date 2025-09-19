use crate::display::JingleDisplayable;
use crate::modeling::State;
use crate::python::resolved_varnode::PythonResolvedVarNode;
use crate::python::sleigh_context::PythonLoadedSleighContext;
use crate::python::z3::ast::PythonAst;
use crate::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle_sleigh::{IndirectVarNode, VarNode};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

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

    /// Create a direct varnode from space name, offset, and size
    ///
    /// # See also
    /// - [Self::indirect_varnode]
    /// - [Self::read_varnode]
    pub fn direct_varnode(
        &self,
        space: &str,
        offset: u64,
        size: usize,
    ) -> PyResult<PythonResolvedVarNode> {
        let vn =
            self.state
                .arch_info()
                .varnode(space, offset, size)
                .ok_or(PyRuntimeError::new_err(format!(
                    "Invalid space name: {space}"
                )))?;
        Ok(PythonResolvedVarNode::from(
            ResolvedVarnode::from(vn).display(self.state.arch_info()),
        ))
    }

    /// Create an indirect varnode from a direct pointer varnode, space name, and access size
    /// The pointer varnode must be direct (i.e. not itself indirect)
    ///
    /// # See also
    /// - [Self::direct_varnode]
    /// - [Self::read_varnode]
    pub fn indirect_varnode(
        &self,
        space: &str,
        pointer: PythonResolvedVarNode,
        access_size_bytes: usize,
    ) -> PyResult<PythonResolvedVarNode> {
        if let ResolvedVarnode::Direct(pointer_location) = pointer.inner.inner() {
            let pointer_space_index = self
                .state
                .arch_info()
                .spaces()
                .iter()
                .position(|s| s.name == space)
                .ok_or(PyRuntimeError::new_err(format!(
                    "Invalid space name: {space}"
                )))?;
            let indirect = IndirectVarNode {
                access_size_bytes,
                pointer_location: pointer_location.clone(),
                pointer_space_index,
            };
            let pointer = self.state.read_varnode_indirect(&indirect)?;
            let resolved = ResolvedIndirectVarNode {
                pointer,
                pointer_location: pointer_location.clone(),
                access_size_bytes,
                pointer_space_idx: pointer_space_index,
            };
            Ok(PythonResolvedVarNode::from(
                ResolvedVarnode::Indirect(resolved).display(self.state.arch_info()),
            ))
        } else {
            Err(PyRuntimeError::new_err(
                "Indirect varnodes must be created with a direct pointer",
            ))
        }
    }

    /// Read a varnode, direct or indirect, from the symbolic state
    ///
    /// # See also
    /// - [Self::direct_varnode]
    /// - [Self::indirect_varnode]
    /// - [Self::read_register]
    /// - [Self::read_ram]
    pub fn read_varnode(&self, varnode: &PythonResolvedVarNode) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            self.state
                .read_resolved(varnode.inner.inner())?
                .try_into_python(py)
        })
    }

    /// Read a named register from the symbolic state
    ///
    /// # See also
    /// - [Self::read_varnode]
    /// - [Self::read_ram]
    pub fn read_register(&self, name: &str) -> PyResult<Py<PyAny>> {
        Python::attach(|py| {
            let vn = self
                .state
                .arch_info()
                .register(name)
                .ok_or(PyRuntimeError::new_err("Queried nonexistent register"))?;
            self.state.read_varnode(vn)?.try_into_python(py)
        })
    }

    /// Read a range of bytes from the default code space in the symbolic state
    ///
    /// The result is a bitvector of size `length * 8` (i.e. `length` bytes)
    ///
    /// # See also
    /// - [Self::read_varnode]
    /// - [Self::read_register]
    pub fn read_ram(&self, offset: u64, length: usize) -> PyResult<Py<PyAny>> {
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
