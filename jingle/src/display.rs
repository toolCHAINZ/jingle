use crate::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle_sleigh::SleighArchInfo;
pub use jingle_sleigh::{JingleDisplay, JingleDisplayWrapper};
use std::fmt::Formatter;
use z3::ast::Ast;

impl JingleDisplay for ResolvedIndirectVarNode {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, ctx: &SleighArchInfo) -> std::fmt::Result {
        write!(
            f,
            "{}({})",
            ctx.get_space(self.pointer_space_idx)
                .ok_or(std::fmt::Error)?
                .name,
            self.pointer.simplify(),
        )
    }
}

impl JingleDisplay for ResolvedVarnode {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, ctx: &SleighArchInfo) -> std::fmt::Result {
        match self {
            ResolvedVarnode::Direct(a) => a.fmt_jingle(f, ctx),
            ResolvedVarnode::Indirect(i) => i.fmt_jingle(f, ctx),
        }
    }
}
