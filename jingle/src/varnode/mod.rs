use jingle_sleigh::{VarNode};
use std::hash::Hash;
use z3::ast::BV;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ResolvedIndirectVarNode {
    pub pointer_space_idx: usize,
    pub pointer: BV,
    pub pointer_location: VarNode,
    pub access_size_bytes: usize,
}

/// This represents a general varnode that has been evaluated in a sequence of instructions.
/// What distinguishes this from a regular VarNode is that, in the case of indirect varnodes,
/// the pointer value has been already evaluated
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ResolvedVarnode {
    Direct(VarNode),
    Indirect(ResolvedIndirectVarNode),
}

impl From<VarNode> for ResolvedVarnode {
    fn from(value: VarNode) -> Self {
        Self::Direct(value)
    }
}
