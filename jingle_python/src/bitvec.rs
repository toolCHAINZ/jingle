use pyo3::prelude::PyAnyMethods;
use pyo3::types::PyModule;
use pyo3::{Bound, IntoPy, IntoPyObject, IntoPyObjectExt, Py, PyAny, PyObject, PyResult, Python};
use z3::ast::{Ast, BV};

pub fn adapt_bv(bv: BV) -> PyResult<Py<PyAny>> {
    Python::with_gil(|py: Python| {
        let z3_mod = PyModule::import(py, "z3")?;
        let ref_class = z3_mod.getattr("BitVecRef")?.into_pyobject(py)?;
        let ctypes = PyModule::import(py, "ctypes")?;
        let ptr_type = ctypes.getattr("c_void_p")?;
        let args = bv.get_z3_ast() as usize;
        let ptr = ptr_type.call1((args,))?;

        let a = ref_class.call1((ptr,))?.into_py_any(py)?;
        Ok(a)
    })
}
