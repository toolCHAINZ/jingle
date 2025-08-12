use crate::display::{JingleDisplay, JingleDisplayable};
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
#[pyclass(unsendable, str)]
pub struct PythonResolvedVarNode {
    pub inner: PythonResolvedVarNodeInner,
}

impl Display for PythonResolvedVarNode {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match &self.inner {
            PythonResolvedVarNodeInner::Direct(d) => d.fmt(f),
            PythonResolvedVarNodeInner::Indirect(i) => i.fmt(f),
        }
    }
}

impl From<JingleDisplay<VarNode>> for PythonResolvedVarNode {
    fn from(value: JingleDisplay<VarNode>) -> Self {
        Self {
            inner: PythonResolvedVarNodeInner::Direct(value),
        }
    }
}

impl From<JingleDisplay<ResolvedVarnode>> for PythonResolvedVarNode {
    fn from(value: JingleDisplay<ResolvedVarnode>) -> Self {
        let inner = value.inner();
        let inner = match inner {
            ResolvedVarnode::Direct(a) => {
                PythonResolvedVarNodeInner::Direct(a.display(value.info()))
            }
            ResolvedVarnode::Indirect(i) => {
                PythonResolvedVarNodeInner::Indirect(i.display(value.info()))
            }
        };
        Self { inner }
    }
}
impl From<JingleDisplay<ResolvedIndirectVarNode>> for PythonResolvedVarNode {
    fn from(value: JingleDisplay<ResolvedIndirectVarNode>) -> Self {
        Self {
            inner: PythonResolvedVarNodeInner::Indirect(value),
        }
    }
}
