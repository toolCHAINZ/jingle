use jingle_sleigh::VarNodeDisplay;
use std::fmt::{Display, Formatter};
use z3::ast::{Ast, BV};

#[derive(Debug, Clone)]
pub struct ResolvedIndirectVarNodeDisplay<'ctx> {
    pub pointer_space_name: String,
    pub pointer: BV<'ctx>,
    pub access_size_bytes: usize,
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
                    i.pointer_space_name,
                    i.pointer.simplify(),
                    i.access_size_bytes
                )
            }
        }
    }
}
