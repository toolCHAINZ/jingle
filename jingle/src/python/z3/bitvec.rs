use crate::python::z3::ast::{TryFromPythonZ3, TryIntoPythonZ3};
use crate::python::z3::get_python_z3;
use pyo3::prelude::PyAnyMethods;
use pyo3::types::PyModule;
use pyo3::{IntoPyObject, IntoPyObjectExt, Py, PyAny, PyResult, Python};
use z3::Context;
use z3::ast::{Ast, BV};
use z3_sys::Z3_ast;

impl<'ctx> TryIntoPythonZ3 for BV<'ctx> {
    fn try_into_python(mut self) -> PyResult<Py<PyAny>> {
        Python::with_gil(|py: Python| {
            let z3 = get_python_z3()?;
            if z3 != self.get_ctx() {
                self = self.translate(z3);
            }
            let z3_mod = PyModule::import(py, "z3")?;
            let ref_class = z3_mod.getattr("BitVecRef")?.into_pyobject(py)?;
            let ctypes = PyModule::import(py, "ctypes")?;
            let ptr_type = ctypes.getattr("c_void_p")?;
            let ast = self.get_z3_ast() as usize;
            let ptr = ptr_type.call1((ast,))?;

            let a = ref_class.call1((ptr,))?.into_py_any(py)?;
            Ok(a)
        })
    }
}

impl<'ctx> TryFromPythonZ3<'ctx> for BV<'ctx> {
    fn try_from_python(py_bv: Py<PyAny>, ctx: &'ctx Context) -> PyResult<Self> {
        Python::with_gil(|py| {
            let z3 = get_python_z3()?;
            let ast = py_bv.getattr(py, "ast")?;
            let ast = ast.getattr(py, "value")?;
            let ast: usize = ast.extract(py)?;
            let ast = ast as Z3_ast;
            let mut bv = unsafe { BV::wrap(z3, ast) };
            if ctx != bv.get_ctx() {
                bv = bv.translate(ctx);
            }
            Ok(bv)
        })
    }
}
