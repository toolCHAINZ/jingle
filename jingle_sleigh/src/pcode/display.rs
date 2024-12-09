use crate::pcode::PcodeOperation;
use std::fmt::{Display, Formatter, LowerHex, UpperHex};
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
        Ok(())
    }
}

impl UpperHex for PcodeOperation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(o) = self.output() {
            write!(f, "{:X} = ", o)?;
        }
        write!(f, "{} ", self.opcode())?;
        let mut args: Vec<String> = vec![];
        for x in self.inputs() {
            args.push(format!("{:X}", x));
        }
        write!(f, "{}", args.join(", "))?;
        Ok(())
    }
}

impl Display for crate::ffi::opcode::bridge::OpCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let d = format!("{:?}", self);
        write!(f, "{}", &d[5..])
    }
}
