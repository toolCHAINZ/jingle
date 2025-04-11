use jingle_sleigh::GeneralizedVarNodeDisplay;
use pyo3::{pyclass, pymethods, PyRef, PyRefMut};

#[pyclass(unsendable)]
pub struct VarNodeIterator {
    vn: Box<dyn Iterator<Item = GeneralizedVarNodeDisplay>>,
}

impl VarNodeIterator {
    pub fn new<T: Iterator<Item = GeneralizedVarNodeDisplay> + 'static>(t: T) -> Self {
        Self { vn: Box::new(t) }
    }
}
#[pymethods]
impl VarNodeIterator {
    pub fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    pub fn __next__(mut slf: PyRefMut<Self>) -> Option<GeneralizedVarNodeDisplay> {
        slf.vn.next()
    }
}
