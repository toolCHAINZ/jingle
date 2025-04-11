use crate::varnode::display::{ResolvedIndirectVarNodeDisplay, ResolvedVarNodeDisplay};
use jingle_sleigh::VarNodeDisplay;
use pyo3::pyclass;
use std::fmt::{Display, Formatter};

#[derive(Clone)]
#[pyclass(unsendable, str)]
pub struct PythonResolvedIndirectVarNode {
    pub inner: ResolvedIndirectVarNodeDisplay<'static>,
}

impl Display for PythonResolvedIndirectVarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

#[derive(Clone)]
#[pyclass(unsendable, str)]
pub enum PythonResolvedVarNode {
    Direct(VarNodeDisplay),
    Indirect(PythonResolvedIndirectVarNode),
}

impl Display for PythonResolvedVarNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PythonResolvedVarNode::Direct(d) => Display::fmt(&d, f),
            PythonResolvedVarNode::Indirect(i) => i.fmt(f),
        }
    }
}

impl From<VarNodeDisplay> for PythonResolvedVarNode {
    fn from(value: VarNodeDisplay) -> Self {
        Self::Direct(value)
    }
}

impl From<PythonResolvedIndirectVarNode> for PythonResolvedVarNode {
    fn from(value: PythonResolvedIndirectVarNode) -> Self {
        Self::Indirect(value)
    }
}

impl From<ResolvedVarNodeDisplay<'static>> for PythonResolvedVarNode {
    fn from(value: ResolvedVarNodeDisplay<'static>) -> Self {
        match value {
            ResolvedVarNodeDisplay::Direct(d) => PythonResolvedVarNode::Direct(d),
            ResolvedVarNodeDisplay::Indirect(a) => {
                PythonResolvedVarNode::Indirect(PythonResolvedIndirectVarNode { inner: a })
            }
        }
    }
}
