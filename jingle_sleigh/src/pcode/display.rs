use crate::pcode::PcodeOperation;
use crate::RegisterManager;
use std::fmt::{Display, Formatter, LowerHex};

pub struct PcodeOperationDisplay<'a, T: RegisterManager> {
    pub(crate) op: PcodeOperation,
    pub(crate) ctx: &'a T,
}

impl<'a, T: RegisterManager> PcodeOperationDisplay<'a, T> {}

impl Display for PcodeOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(o) = self.output() {
            write!(f, "{} = ", o)?;
        }
        write!(f, "{} ", self.opcode())?;
        let mut args: Vec<String> = vec![];
        for x in self.inputs() {
            args.push(format!("{}", x));
        }
        write!(f, "{}", args.join(", "))?;
        Ok(())
    }
}

impl LowerHex for PcodeOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(o) = self.output() {
            write!(f, "{:x} = ", o)?;
        }
        write!(f, "{} ", self.opcode())?;
        let mut args: Vec<String> = vec![];
        for x in self.inputs() {
            args.push(format!("{:x}", x));
        }
        write!(f, "{}", args.join(", "))?;
        Ok(())    }
}

impl Display for crate::ffi::opcode::bridge::OpCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let d = format!("{:?}", self);
        write!(f, "{}", &d[5..])
    }
}
