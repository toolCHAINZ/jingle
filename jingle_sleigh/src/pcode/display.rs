use crate::pcode::PcodeOperation;
use crate::pcode::PcodeOperation::{
    Branch, BranchInd, CBranch, Call, CallInd, CallOther, Copy, Int2Comp, IntAdd, IntAnd, IntCarry,
    IntEqual, IntLeftShift, IntLess, IntLessEqual, IntNegate, IntNotEqual, IntOr, IntRightShift,
    IntSExt, IntSignedBorrow, IntSignedCarry, IntSignedLess, IntSignedLessEqual, IntSub, IntXor,
    IntZExt, Load, PopCount, Return, Store,
};
use crate::space::SpaceManager;
use std::fmt::{Display, Formatter};
use crate::RegisterManager;

pub struct PcodeOperationDisplay<'a, T: RegisterManager> {
    pub(crate) op: PcodeOperation,
    pub(crate) ctx: &'a T,
}

impl<'a, T: RegisterManager> PcodeOperationDisplay<'a, T> {}

impl<'a, T> Display for PcodeOperationDisplay<'a, T>
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


impl Display for crate::ffi::opcode::bridge::OpCode{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let d = format!("{:?}", self);
        write!(f, "{}", d[5..].to_string())
    }
}