use crate::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle_sleigh::{SpaceInfo, VarNode, VarNodeDisplay};
use std::fmt::{Display, Formatter};
use z3::ast::{Ast, BV};

#[derive(Debug, Clone)]
pub struct ResolvedIndirectVarNodeDisplay {
    pub pointer_space_info: SpaceInfo,
    pub pointer: BV,
    pub access_size_bytes: usize,
    pub pointer_location: VarNode,
}

#[derive(Debug, Clone)]
pub enum ResolvedVarNodeDisplay {
    Direct(VarNodeDisplay),
    Indirect(ResolvedIndirectVarNodeDisplay),
}

impl Display for ResolvedIndirectVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}[{}]:{}",
            self.pointer_space_info.name,
            self.pointer.simplify(),
            self.access_size_bytes
        )
    }
}

impl Display for ResolvedVarNodeDisplay {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            ResolvedVarNodeDisplay::Direct(d) => d.fmt(f),
            ResolvedVarNodeDisplay::Indirect(i) => i.fmt(f),
        }
    }
}

impl From<ResolvedVarNodeDisplay> for ResolvedVarnode {
    fn from(value: ResolvedVarNodeDisplay) -> Self {
        match value {
            ResolvedVarNodeDisplay::Direct(a) => ResolvedVarnode::Direct(a.into()),
            ResolvedVarNodeDisplay::Indirect(a) => {
                ResolvedVarnode::Indirect(ResolvedIndirectVarNode::from(a))
            }
        }
    }
}
