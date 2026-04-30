use crate::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle_sleigh::display::JingleDisplayWrapper;
use jingle_sleigh::{JingleDisplay, SleighArchInfo, VarNode};
use pyo3::pyclass;
use std::fmt::{Display, Formatter};

#[derive(Clone)]
pub enum PythonResolvedVarNodeInner {
    Direct(VarNode, SleighArchInfo),
    Indirect(ResolvedIndirectVarNode, SleighArchInfo),
}
#[derive(Clone)]
#[pyclass(unsendable, str, name = "ResolvedVarNode")]
pub struct PythonResolvedVarNode {
    pub inner: ResolvedVarnode,
    info: SleighArchInfo,
}

impl Display for PythonResolvedVarNode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.inner.fmt_jingle(f, &self.info)
    }
}

impl From<JingleDisplayWrapper<'_, ResolvedVarnode>> for PythonResolvedVarNode {
    fn from(value: JingleDisplayWrapper<ResolvedVarnode>) -> Self {
        Self {
            inner: value.inner().clone(),
            info: value.info().clone(),
        }
    }
}
