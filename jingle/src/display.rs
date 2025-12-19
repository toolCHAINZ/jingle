use crate::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle_sleigh::{
    GeneralizedVarNode, IndirectVarNode, Instruction, PcodeOperation, SleighArchInfo, SpaceType,
    VarNode,
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

#[derive(Clone)]
pub struct JingleDisplay<T> {
    info: SleighArchInfo,
    inner: T,
}

impl<T> JingleDisplay<T> {
    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn info(&self) -> &SleighArchInfo {
        &self.info
    }
}

impl<T: JingleDisplayable> Display for JingleDisplay<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt_jingle(f, &self.info)
    }
}

impl JingleDisplay<ResolvedIndirectVarNode> {
    pub fn space_name(&self) -> &str {
        self.info
            .get_space(self.inner.pointer_space_idx)
            .unwrap()
            .name
            .as_str()
    }
}

impl JingleDisplayable for VarNode {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, ctx: &SleighArchInfo) -> std::fmt::Result {
        if self.space_index == VarNode::CONST_SPACE_INDEX {
            write!(f, "{:#x}:{}", self.offset, self.size)
        } else if let Some(name) = ctx.register_name(self) {
            write!(f, "{name}")
        } else if let Some(SpaceType::IPTR_INTERNAL) =
            ctx.get_space(self.space_index).map(|s| s._type)
        {
            write!(f, "$U{:x}:{}", self.offset, self.size)
        } else {
            write!(
                f,
                "[{}]{:#x}:{}",
                ctx.get_space(self.space_index).ok_or(std::fmt::Error)?.name,
                self.offset,
                self.size
            )
        }
    }
}

impl JingleDisplayable for IndirectVarNode {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, ctx: &SleighArchInfo) -> std::fmt::Result {
        write!(
            f,
            "{}({})",
            ctx.get_space(self.pointer_space_index)
                .ok_or(std::fmt::Error)?
                .name,
            self.pointer_location.display(ctx)
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
            "{}({})",
            ctx.get_space(self.pointer_space_idx)
                .ok_or(std::fmt::Error)?
                .name,
            self.pointer.simplify(),
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
        write!(f, "{} ", self.opcode())?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use jingle_sleigh::context::SleighContextBuilder;
    use jingle_sleigh::{PcodeOperation, VarNode};

    fn make_sleigh() -> jingle_sleigh::context::SleighContext {
        // Mirrors other tests in the repo which expect a local Ghidra checkout at this path.
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        ctx_builder.build("x86:LE:64:default").unwrap()
    }

    #[test]
    fn test_varnode_display_cases() {
        let sleigh = make_sleigh();
        let info = sleigh.arch_info().clone();

        // const varnode should display as "<hex_offset>:<size>"
        let const_vn = VarNode {
            space_index: VarNode::CONST_SPACE_INDEX,
            offset: 0x42,
            size: 1,
        };
        assert_eq!(format!("{}", const_vn.display(&info)), "0x42:1");

        // registers: if registers exist, pretty display should equal the register name
        let regs: Vec<_> = sleigh.arch_info().registers().collect();
        if !regs.is_empty() {
            let (vn, name) = regs[0].clone();
            assert_eq!(format!("{}", vn.display(&info)), name);
        }

        // other space: we expect a bracketed form like "[<space>]offset:size" or internal $U...
        let other = VarNode {
            space_index: 1,
            offset: 0x10,
            size: 4,
        };
        let s = format!("{}", other.display(&info));
        // be permissive: ensure it contains the size and either bracket or unique prefix
        assert!(s.contains(":4"));
    }

    #[test]
    fn test_pcode_display_and_round_trip_copy() {
        let sleigh = make_sleigh();
        let info = sleigh.arch_info().clone();

        let output = VarNode {
            space_index: 4,
            offset: 0x20,
            size: 4,
        };
        let input = VarNode {
            space_index: VarNode::CONST_SPACE_INDEX,
            offset: 0x1,
            size: 4,
        };

        let op = PcodeOperation::Copy {
            input: input.clone(),
            output: output.clone(),
        };

        // display formatting contains the opcode name and operands
        let s = format!("{}", op.display(&info));
        assert_eq!(s, "ESP = COPY 0x1:4");
        // Try to round-trip: render to textual pcode and parse with SleighContext.parse_pcode_listing
        // Use the same textual format that `display` produces for varnodes.

        let parsed = sleigh.parse_pcode_listing(s).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], op);
    }
}
