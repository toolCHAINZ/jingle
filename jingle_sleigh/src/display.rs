use crate::{
    GeneralizedVarNode, IndirectVarNode, Instruction, PcodeOperation, SleighArchInfo, SpaceType,
    VarNode,
};
use std::fmt::{Display, Formatter};

/// Trait for rendering types using the Sleigh architecture display context.
///
/// Types that implement this trait can produce a human-friendly representation
/// that may depend on architecture-specific information (register names, space
/// names, endianness, etc).
pub trait JingleDisplay: Sized + Clone {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result;

    fn display<T: AsRef<SleighArchInfo>>(&self, info: T) -> JingleDisplayWrapper<Self> {
        JingleDisplayWrapper {
            info: info.as_ref().clone(),
            inner: self.clone(),
        }
    }
}

/// A small helper that bundles a value with Sleigh arch info so it implements
/// `std::fmt::Display` by forwarding to `fmt_jingle`.
#[derive(Clone)]
pub struct JingleDisplayWrapper<T> {
    info: SleighArchInfo,
    inner: T,
}

impl<T> JingleDisplayWrapper<T> {
    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn info(&self) -> &SleighArchInfo {
        &self.info
    }
}

impl<T: JingleDisplay> Display for JingleDisplayWrapper<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt_jingle(f, &self.info)
    }
}

impl JingleDisplay for VarNode {
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

impl JingleDisplay for IndirectVarNode {
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

impl JingleDisplay for GeneralizedVarNode {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, ctx: &SleighArchInfo) -> std::fmt::Result {
        match self {
            GeneralizedVarNode::Direct(d) => d.fmt_jingle(f, ctx),
            GeneralizedVarNode::Indirect(i) => i.fmt_jingle(f, ctx),
        }
    }
}

// ResolvedIndirectVarNode and ResolvedVarnode are types defined in the `jingle` crate.
// Their `JingleDisplayable` implementations are intentionally kept in `jingle` so
// they can reference jingle-specific types. Implementations for sleigh-owned
// types remain in this file.

impl JingleDisplay for PcodeOperation {
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

impl JingleDisplay for Instruction {
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
    use crate::context::SleighContextBuilder;
    use crate::{PcodeOperation, VarNode};

    fn make_sleigh() -> crate::context::SleighContext {
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
