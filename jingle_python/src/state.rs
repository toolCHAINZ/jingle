use jingle::modeling::State;
use pyo3::pyclass;

#[pyclass(unsendable)]
pub struct PythonState {
    state: State<'static>,
}
