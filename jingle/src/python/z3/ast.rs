use crate::python::z3::get_python_z3;
use pyo3::prelude::{PyAnyMethods, PyModule};
use pyo3::{IntoPyObject, IntoPyObjectExt, Py, PyAny, PyResult, Python};
use z3::ast::{Ast, BV, Bool};
use z3::{Context, Translate};
use z3_sys::Z3_ast;

pub trait TryFromPythonZ3: Sized {
    fn try_from_python(py: Py<PyAny>) -> PyResult<Self>;
}

pub trait TryIntoPythonZ3: Sized {
    fn try_into_python(self) -> PyResult<Py<PyAny>>;
}

pub trait PythonAst: Sized + Ast {
    const CLASS_NAME: &'static str;
    fn try_into_python(&self) -> PyResult<Py<PyAny>> {
        Python::with_gil(|py: Python| {
            let z3 = get_python_z3()?;
            let translated = self.translate(&z3);
            let z3_mod = PyModule::import(py, "z3")?;
            let ref_class = z3_mod.getattr(Self::CLASS_NAME)?.into_pyobject(py)?;
            let ctypes = PyModule::import(py, "ctypes")?;
            let ptr_type = ctypes.getattr("c_void_p")?;
            let ast = translated.get_z3_ast() as usize;
            let ptr = ptr_type.call1((ast,))?;

            let a = ref_class.call1((ptr,))?.into_py_any(py)?;
            Ok(a)
        })
    }
    fn try_from_python(py_bv: Py<PyAny>) -> PyResult<Self> {
        Python::with_gil(|py| {
            let z3 = get_python_z3()?;
            let ast = py_bv.getattr(py, "ast")?;
            let ast = ast.getattr(py, "value")?;
            let ast: usize = ast.extract(py)?;
            let ast = ast as Z3_ast;
            let p_ast = unsafe { Self::wrap(&z3, ast) };
            let translated = p_ast.translate(&Context::thread_local());
            Ok(translated)
        })
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
