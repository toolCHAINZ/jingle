mod display;

use crate::error::JingleError;
use crate::error::JingleError::UnmodeledSpace;
use crate::varnode::display::{ResolvedIndirectVarNodeDisplay, ResolvedVarNodeDisplay};
use jingle_sleigh::RegisterManager;
use jingle_sleigh::VarNode;
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

impl ResolvedVarnode<'_> {
    pub fn display<T: RegisterManager>(
        &self,
        ctx: &T,
    ) -> Result<ResolvedVarNodeDisplay, JingleError> {
        match self {
            ResolvedVarnode::Direct(d) => Ok(ResolvedVarNodeDisplay::Direct(d.display(ctx)?)),
            ResolvedVarnode::Indirect(i) => Ok(ResolvedVarNodeDisplay::Indirect(
                ResolvedIndirectVarNodeDisplay {
                    pointer_space_name: ctx
                        .get_space_info(i.pointer_space_idx)
                        .map(|o| o.name.clone())
                        .ok_or(UnmodeledSpace)?,
                    pointer: i.pointer.clone(),
                    access_size_bytes: i.access_size_bytes,
                },
            )),
        }
    }
}
