use pyo3::{Py, PyAny, PyResult};
use z3::Context;

pub trait TryFromPythonZ3: Sized {
    fn try_from_python(py: Py<PyAny>) -> PyResult<Self>;
}

pub trait TryIntoPythonZ3: Sized {
    fn try_into_python(self) -> PyResult<Py<PyAny>>;
}
