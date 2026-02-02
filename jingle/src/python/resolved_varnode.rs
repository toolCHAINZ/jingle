use crate::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle_sleigh::VarNode;
use jingle_sleigh::display::JingleDisplayWrapper;
use pyo3::pyclass;
use std::fmt::{Display, Formatter};

#[derive(Clone)]
pub enum PythonResolvedVarNodeInner {
    Direct(JingleDisplayWrapper<VarNode>),
    Indirect(JingleDisplayWrapper<ResolvedIndirectVarNode>),
}
#[derive(Clone)]
#[pyclass(unsendable, str, name = "ResolvedVarNode")]
pub struct PythonResolvedVarNode {
    pub inner: JingleDisplayWrapper<ResolvedVarnode>,
}

impl Display for PythonResolvedVarNode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl From<JingleDisplayWrapper<ResolvedVarnode>> for PythonResolvedVarNode {
    fn from(value: JingleDisplayWrapper<ResolvedVarnode>) -> Self {
        Self { inner: value }
    }
}
