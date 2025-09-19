use crate::display::JingleDisplay;
use crate::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle_sleigh::VarNode;
use pyo3::pyclass;
use std::fmt::{Display, Formatter};

#[derive(Clone)]
pub enum PythonResolvedVarNodeInner {
    Direct(JingleDisplay<VarNode>),
    Indirect(JingleDisplay<ResolvedIndirectVarNode>),
}
#[derive(Clone)]
#[pyclass(unsendable, str, name="ResolvedVarNode")]
pub struct PythonResolvedVarNode {
    pub inner: JingleDisplay<ResolvedVarnode>,
}

impl Display for PythonResolvedVarNode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl From<JingleDisplay<ResolvedVarnode>> for PythonResolvedVarNode {
    fn from(value: JingleDisplay<ResolvedVarnode>) -> Self {
        Self { inner: value }
    }
}
