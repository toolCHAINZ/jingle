use crate::python::resolved_varnode::PythonResolvedVarNode;
use pyo3::{pyclass, pymethods, PyRef, PyRefMut};

#[pyclass(unsendable)]
pub struct VarNodeIterator {
    vn: Box<dyn Iterator<Item = PythonResolvedVarNode>>,
}

impl VarNodeIterator {
    pub fn new<T: Iterator<Item = PythonResolvedVarNode> + 'static>(t: T) -> Self {
        Self { vn: Box::new(t) }
    }
}
#[pymethods]
impl VarNodeIterator {
    pub fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<Self>) -> Option<PythonResolvedVarNode> {
        slf.vn.next()
    }
}

impl Iterator for VarNodeIterator {
    type Item = PythonResolvedVarNode;

    fn next(&mut self) -> Option<Self::Item> {
        self.vn.next()
    }
}
