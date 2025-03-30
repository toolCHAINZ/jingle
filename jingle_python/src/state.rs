use crate::bitvec::adapt_bv;
use crate::jingle_context::PythonJingleContext;
use jingle::modeling::State;
use jingle::sleigh::VarNode;
use pyo3::prelude::*;

#[pyclass(unsendable, name="State")]
pub struct PythonState {
    state: State<'static>,
}

#[pymethods]
impl PythonState {
    #[new]
    pub fn new(j: PyRef<PythonJingleContext>) -> PyResult<PythonState> {
        Ok(PythonState {
            state: State::new(&j.context),
        })
    }

    pub fn read_varnode(&self, varnode: &VarNode) -> PyResult<PyObject> {
        adapt_bv(self.state.read_varnode(varnode)?)
    }
}
