use jingle_sleigh::VarNode;
use std::hash::Hash;
use z3::ast::BV;
use z3::{Context, Translate};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ResolvedIndirectVarNode {
    pub pointer_space_idx: usize,
    pub pointer: BV,
    pub pointer_location: VarNode,
    pub access_size_bytes: usize,
}

unsafe impl Translate for ResolvedIndirectVarNode {
    fn translate(&self, dest: &Context) -> Self {
        Self {
            pointer_space_idx: self.pointer_space_idx.clone(),
            pointer_location: self.pointer_location.clone(),
            access_size_bytes: self.access_size_bytes,
            pointer: self.pointer.translate(dest)
        }
    }
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

unsafe impl Translate for ResolvedVarnode {
    fn translate(&self, dest: &Context) -> Self {
        match self {
            ResolvedVarnode::Direct(a) => {ResolvedVarnode::Direct(a.clone())}
            ResolvedVarnode::Indirect(i) => {ResolvedVarnode::Indirect(i.translate(dest))}
        }
    }
}
