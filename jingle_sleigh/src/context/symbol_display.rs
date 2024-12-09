use crate::context::SleighContext;
use crate::varnode::display::symbolized::SymbolizedGeneralVarNodeDisplay;
use crate::{GeneralizedVarNode, PcodeOperation};
use std::fmt::{Display, Formatter, LowerHex, UpperHex};

pub struct SymbolizedPcodeOperationDisplay<'ctx, 'op> {
    pub(crate) sleigh: &'ctx SleighContext,
    pub(crate) operation: &'op PcodeOperation,
}

impl SymbolizedPcodeOperationDisplay<'_, '_> {
    fn map_gen(&self, gen: &GeneralizedVarNode) -> SymbolizedGeneralVarNodeDisplay {
        match gen {
            GeneralizedVarNode::Direct(o) => self.sleigh.apply_symbols_to_varnode(o).into(),
            GeneralizedVarNode::Indirect(i) => {
                self.sleigh.apply_symbols_to_indirect_varnode(i).into()
            }
        }
    }
    fn output(&self) -> Option<SymbolizedGeneralVarNodeDisplay> {
        self.operation.output().map(|out| self.map_gen(&out))
    }

    fn inputs(&self) -> Vec<SymbolizedGeneralVarNodeDisplay> {
        self.operation.inputs().iter().map(|out| self.map_gen(&out)).collect()
    }
}

impl Display for SymbolizedPcodeOperationDisplay<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(o) = self.output() {
            write!(f, "{} = ", o)?;
        }
        write!(f, "{} ", self.operation.opcode())?;
        let mut args: Vec<String> = vec![];
        for x in self.inputs() {
            args.push(format!("{}", x));
        }
        write!(f, "{}", args.join(", "))?;
        Ok(())
    }
}

impl LowerHex for SymbolizedPcodeOperationDisplay<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(o) = self.output() {
            write!(f, "{:x} = ", o)?;
        }
        write!(f, "{} ", self.operation.opcode())?;
        let mut args: Vec<String> = vec![];
        for x in self.inputs() {
            args.push(format!("{:x}", x));
        }
        write!(f, "{}", args.join(", "))?;
        Ok(())
    }
}

impl UpperHex for SymbolizedPcodeOperationDisplay<'_, '_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(o) = self.output() {
            write!(f, "{:X} = ", o)?;
        }
        write!(f, "{} ", self.operation.opcode())?;
        let mut args: Vec<String> = vec![];
        for x in self.inputs() {
            args.push(format!("{:X}", x));
        }
        write!(f, "{}", args.join(", "))?;
        Ok(())
    }
}
