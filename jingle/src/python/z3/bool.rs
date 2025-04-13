use crate::python::z3::ast::{TryFromPythonZ3, TryIntoPythonZ3};
use crate::python::z3::get_python_z3;
use pyo3::prelude::PyAnyMethods;
use pyo3::types::PyModule;
use pyo3::{IntoPyObject, IntoPyObjectExt, Py, PyAny, PyResult, Python};
use z3::ast::{Ast, Bool, BV};
use z3_sys::Z3_ast;

impl TryIntoPythonZ3 for Bool<'static> {
    fn try_into_python(self) -> PyResult<Py<PyAny>> {
        Python::with_gil(|py: Python| {
            let z3 = get_python_z3()?;
            self.translate(z3);
            let z3_mod = PyModule::import(py, "z3")?;
            let ref_class = z3_mod.getattr("BoolRef")?.into_pyobject(py)?;
            let ctypes = PyModule::import(py, "ctypes")?;
            let ptr_type = ctypes.getattr("c_void_p")?;
            let ast = self.get_z3_ast() as usize;
            let ptr = ptr_type.call1((ast,))?;

            let a = ref_class.call1((ptr,))?.into_py_any(py)?;
            Ok(a)
        })
    }
}

impl TryFromPythonZ3 for Bool<'static> {
    fn try_from_python(py_bv: Py<PyAny>) -> PyResult<Self> {
        Python::with_gil(|py| {
            let z3 = get_python_z3()?;

            let ast = py_bv.getattr(py, "ast")?;
            let ast = ast.getattr(py, "value")?;
            let ast: usize = ast.extract(py)?;
            let ast = ast as Z3_ast;
            unsafe { Ok(Bool::wrap(z3, ast)) }
        })
    }
}
