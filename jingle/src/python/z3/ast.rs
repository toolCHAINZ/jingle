use crate::python::z3::get_python_z3;
use pyo3::prelude::{PyAnyMethods, PyModule};
use pyo3::{IntoPyObject, IntoPyObjectExt, Py, PyAny, PyResult, Python};
use z3::Context;
use z3::ast::{Ast, BV, Bool};
use z3_sys::Z3_ast;

pub trait TryFromPythonZ3: Sized {
    fn try_from_python(obj: Py<PyAny>, py: Python) -> PyResult<Self>;
}

pub trait TryIntoPythonZ3: Sized {
    fn try_into_python(self) -> PyResult<Py<PyAny>>;
}

pub trait PythonAst: Sized + Ast {
    const CLASS_NAME: &'static str;
    fn try_into_python(&self, py: Python) -> PyResult<Py<PyAny>> {
        let python_z3 = get_python_z3(py)?;
        let self_ast = self.get_z3_ast();
        let self_ctx = self.get_ctx().get_z3_context();
        let translated_ast = unsafe { z3_sys::Z3_translate(self_ctx, self_ast, python_z3) };
        let z3_mod = PyModule::import(py, "z3")?;
        let ref_class = z3_mod.getattr(Self::CLASS_NAME)?.into_pyobject(py)?;
        let ctypes = PyModule::import(py, "ctypes")?;
        let ptr_type = ctypes.getattr("c_void_p")?;
        let ptr = ptr_type.call1((translated_ast as usize,))?;
        let a = ref_class.call1((ptr,))?.into_py_any(py)?;
        Ok(a)
    }
    fn try_from_python(py_bv: Py<PyAny>, py: Python) -> PyResult<Self> {
        let py_z3 = get_python_z3(py)?;
        let our_z3 = Context::thread_local().get_z3_context();
        let raw = py_bv.call_method0(py, "as_ast")?;

        let addr: usize = raw
            .extract(py) // int case
            .or_else(|_| raw.getattr(py, "value")?.extract(py))?; // ctypes.c_void_p.value case
        let ast = addr as Z3_ast;
        let translated_ast = unsafe { z3_sys::Z3_translate(py_z3, ast, our_z3) };
        let p_ast = unsafe { Self::wrap(&Context::thread_local(), translated_ast) };
        Ok(p_ast)
    }
}

macro_rules! python_ast {
    ($t:ty = $l:literal) => {
        impl PythonAst for $t {
            const CLASS_NAME: &'static str = $l;
        }
    };
}

python_ast!(Bool = "BoolRef");
python_ast!(BV = "BitVecRef");
