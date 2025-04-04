use crate::bitvec::adapt_bv;
use jingle::modeling::State;
use jingle::varnode::ResolvedVarnode;
use pyo3::{pyclass, pymethods, Py, PyAny, PyRef, PyRefMut};

#[pyclass(unsendable)]
pub struct VarNodeIterator {
    state: State<'static>,
    vn: Box<dyn Iterator<Item = ResolvedVarnode<'static>>>,
}

impl VarNodeIterator {
    pub fn new<T: Iterator<Item = ResolvedVarnode<'static>> + 'static>(
        state: State<'static>,
        t: T,
    ) -> Self {
        Self {
            state,
            vn: Box::new(t),
        }
    }
}
#[pymethods]
impl VarNodeIterator {
    pub fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<Self>) -> Option<Py<PyAny>> {
        let vn = slf.vn.next()?;
        let vn = slf.state.read_resolved(&vn).ok()?;
        let bv = adapt_bv(vn).ok()?;
        Some(bv)
    }
}
