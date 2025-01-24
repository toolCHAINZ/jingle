use crate::pcode::PcodeOperation;
use crate::RegisterManager;
use std::fmt::{Display, Formatter};

pub struct PcodeOperationDisplay<'a, T: RegisterManager> {
    pub(crate) op: PcodeOperation,
    pub(crate) ctx: &'a T,
}

impl<T: RegisterManager> PcodeOperationDisplay<'_, T> {}

impl<T> Display for PcodeOperationDisplay<'_, T>
where
    T: RegisterManager,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(o) = self.op.output() {
            write!(f, "{} = ", o.display(self.ctx)?)?;
        }
        write!(f, "{} ", self.op.opcode())?;
        let mut args: Vec<String> = vec![];
        for x in self.op.inputs() {
            args.push(format!("{}", x.display(self.ctx)?));
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
