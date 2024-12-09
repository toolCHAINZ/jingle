mod display;

use jingle_sleigh::SharedSpaceInfo;
use jingle_sleigh::VarNode;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use z3::ast::BV;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ResolvedIndirectVarNode<'ctx> {
    pub pointer_space: SharedSpaceInfo,
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

impl Display for ResolvedVarnode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolvedVarnode::Direct(d) => {
                write!(f, "{}", d)
            }
            ResolvedVarnode::Indirect(i) => {
                write!(f, "{}", i)
            }
        }
    }
}

impl Display for ResolvedIndirectVarNode<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "*({}[{}]:{})",
            self.pointer_space.index, self.pointer_location, self.access_size_bytes
        )
    }
}
