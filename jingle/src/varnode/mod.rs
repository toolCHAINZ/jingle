pub mod display;

use crate::error::JingleError;
use crate::error::JingleError::UnmodeledSpace;
use crate::varnode::display::{ResolvedIndirectVarNodeDisplay, ResolvedVarNodeDisplay};
use jingle_sleigh::{ArchInfoProvider, VarNode};
use std::hash::Hash;
use z3::ast::BV;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ResolvedIndirectVarNode<'ctx> {
    pub pointer_space_idx: usize,
    pub pointer: BV<'ctx>,
    pub pointer_location: VarNode,
    pub access_size_bytes: usize,
}

/// This represents a general varnode that has been evaluated in a sequence of instructions.
/// What distinguishes this from a regular VarNode is that, in the case of indirect varnodes,
/// the pointer value has been already evaluated
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ResolvedVarnode<'ctx> {
    Direct(VarNode),
    Indirect(ResolvedIndirectVarNode<'ctx>),
}

impl<'a> ResolvedVarnode<'a> {
    pub fn display<T: ArchInfoProvider>(
        &self,
        ctx: &T,
    ) -> Result<ResolvedVarNodeDisplay<'a>, JingleError> {
        match self {
            ResolvedVarnode::Direct(d) => Ok(ResolvedVarNodeDisplay::Direct(d.display(ctx)?)),
            ResolvedVarnode::Indirect(i) => Ok(ResolvedVarNodeDisplay::Indirect(
                ResolvedIndirectVarNodeDisplay {
                    pointer_space_info: ctx
                        .get_space_info(i.pointer_space_idx)
                        .cloned()
                        .ok_or(UnmodeledSpace)?,
                    pointer: i.pointer.clone(),
                    access_size_bytes: i.access_size_bytes,
                    pointer_location: i.pointer_location.clone(),
                },
            )),
        }
    }
}

impl<'a> From<&ResolvedIndirectVarNodeDisplay<'a>> for ResolvedIndirectVarNode<'a> {
    fn from(value: &ResolvedIndirectVarNodeDisplay<'a>) -> Self {
        ResolvedIndirectVarNode {
            pointer: value.pointer.clone(),
            access_size_bytes: value.access_size_bytes,
            pointer_space_idx: value.pointer_space_info.index,
            pointer_location: value.pointer_location.clone(),
        }
    }
}

impl<'a> From<ResolvedIndirectVarNodeDisplay<'a>> for ResolvedIndirectVarNode<'a> {
    fn from(value: ResolvedIndirectVarNodeDisplay<'a>) -> Self {
        ResolvedIndirectVarNode::from(&value)
    }
}

impl From<VarNode> for ResolvedVarnode<'_>{
    fn from(value: VarNode) -> Self {
        Self::Direct(value)
    }
}