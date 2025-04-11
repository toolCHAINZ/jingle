use crate::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle_sleigh::{SpaceInfo, VarNode, VarNodeDisplay};
use std::fmt::{Display, Formatter};
use z3::ast::{Ast, BV};

#[derive(Debug, Clone)]
pub struct ResolvedIndirectVarNodeDisplay<'ctx> {
    pub pointer_space_info: SpaceInfo,
    pub pointer: BV<'ctx>,
    pub access_size_bytes: usize,
    pub pointer_location: VarNode,
}

#[derive(Debug, Clone)]
pub enum ResolvedVarNodeDisplay<'ctx> {
    Direct(VarNodeDisplay),
    Indirect(ResolvedIndirectVarNodeDisplay<'ctx>),
}

impl Display for ResolvedVarNodeDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedVarNodeDisplay::Direct(d) => d.fmt(f),
            ResolvedVarNodeDisplay::Indirect(i) => {
                write!(
                    f,
                    "{}[{}]:{}",
                    i.pointer_space_info.name,
                    i.pointer.simplify(),
                    i.access_size_bytes
                )
            }
        }
    }
}

impl<'a> From<ResolvedVarNodeDisplay<'a>> for ResolvedVarnode<'a> {
    fn from(value: ResolvedVarNodeDisplay<'a>) -> Self {
        match value {
            ResolvedVarNodeDisplay::Direct(a) => ResolvedVarnode::Direct(a.into()),
            ResolvedVarNodeDisplay::Indirect(a) => {
                ResolvedVarnode::Indirect(ResolvedIndirectVarNode::from(a))
            }
        }
    }
}
