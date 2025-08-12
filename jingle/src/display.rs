use crate::context::SleighArchInfo;
use crate::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle_sleigh::{
    ArchInfoProvider, GeneralizedVarNode, IndirectVarNode, Instruction, PcodeOperation, VarNode,
};
use std::fmt::{Display, Formatter};
use z3::ast::Ast;

pub trait JingleDisplayable: Sized + Clone {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result;

    fn display(&self, info: &SleighArchInfo) -> JingleDisplay<Self> {
        JingleDisplay {
            info: info.clone(),
            inner: self.clone(),
        }
    }
}

pub struct JingleDisplay<T> {
    info: SleighArchInfo,
    inner: T,
}

impl<T: JingleDisplayable> Display for JingleDisplay<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt_jingle(f, &self.info)
    }
}

impl JingleDisplayable for VarNode {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, ctx: &SleighArchInfo) -> std::fmt::Result {
        if self.space_index == VarNode::CONST_SPACE_INDEX {
            write!(f, "{:x}:{:x}", self.offset, self.size)
        } else {
            if let Some(name) = ctx.get_register_name(self) {
                write!(f, "{}", name)
            } else {
                write!(
                    f,
                    "{}[{:x}]:{:x}",
                    ctx.get_space_info(self.space_index)
                        .ok_or(std::fmt::Error::default())?
                        .name,
                    self.offset,
                    self.size
                )
            }
        }
    }
}

impl JingleDisplayable for IndirectVarNode {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, ctx: &SleighArchInfo) -> std::fmt::Result {
        write!(
            f,
            "*({}[{}]:{})",
            ctx.get_space_info(self.pointer_space_index)
                .ok_or(std::fmt::Error::default())?
                .name,
            self.pointer_location,
            self.access_size_bytes
        )
    }
}

impl JingleDisplayable for GeneralizedVarNode {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, ctx: &SleighArchInfo) -> std::fmt::Result {
        match self {
            GeneralizedVarNode::Direct(d) => d.fmt_jingle(f, ctx),
            GeneralizedVarNode::Indirect(i) => i.fmt_jingle(f, ctx),
        }
    }
}

impl JingleDisplayable for ResolvedIndirectVarNode {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, ctx: &SleighArchInfo) -> std::fmt::Result {
        write!(
            f,
            "{}[{}]:{}",
            ctx.get_space_info(self.pointer_space_idx)
                .ok_or(std::fmt::Error::default())?
                .name,
            self.pointer.simplify(),
            self.access_size_bytes
        )
    }
}

impl JingleDisplayable for ResolvedVarnode {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, ctx: &SleighArchInfo) -> std::fmt::Result {
        match self {
            ResolvedVarnode::Direct(a) => a.fmt_jingle(f, ctx),
            ResolvedVarnode::Indirect(i) => i.fmt_jingle(f, ctx),
        }
    }
}

impl JingleDisplayable for PcodeOperation {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, ctx: &SleighArchInfo) -> std::fmt::Result {
        if let Some(o) = self.output() {
            write!(f, "{} = ", o.display(ctx))?;
        }
        write!(f, "{:?} ", self.opcode())?;
        let mut args: Vec<String> = vec![];
        for x in self.inputs() {
            args.push(format!("{}", x.display(ctx)));
        }
        write!(f, "{}", args.join(", "))?;
        Ok(())
    }
}

impl JingleDisplayable for Instruction {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, ctx: &SleighArchInfo) -> std::fmt::Result {
        writeln!(f, "{} {}", self.disassembly.mnemonic, self.disassembly.args)?;
        for x in &self.ops {
            writeln!(f, "\t{}", x.display(ctx))?;
        }
        Ok(())
    }
}
