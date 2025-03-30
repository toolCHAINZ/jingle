use pyo3::prelude::PyAnyMethods;
use pyo3::types::PyModule;
use pyo3::{Bound, IntoPyObject, PyAny, PyObject, PyResult, Python, ToPyObject};
use z3::ast::{Ast, BV};

pub fn adapt_bv<'a>(bv: BV) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        let z3_mod = PyModule::import(py, "z3")?;
        let ref_class = z3_mod.getattr("BitVecRef")?.into_pyobject(py)?;
        let ast_class = z3_mod.getattr("Ast")?.into_pyobject(py)?;
        let ctypes = PyModule::import(py, "ctypes")?;
        let ptr_type = ctypes.getattr("c_void_p")?;
        let args = (bv.get_z3_ast() as usize);
        let ptr = ptr_type.call1((args,))?;

        let ast_obj = ast_class.call1((args,))?.to_object(py);
        let a = ref_class.call1((ptr,))?.to_object(py);
        Ok(a)
    })
}
