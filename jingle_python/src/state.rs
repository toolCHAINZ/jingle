use jingle::modeling::State;
use pyo3::pyclass;

#[pyclass]
pub struct PythonState {
    state: State<'static>,
}
