use crate::pcode::PcodeOperation;
use crate::pcode::PcodeOperation::{
    Branch, BranchInd, CBranch, Call, CallInd, CallOther, Copy, Int2Comp, IntAdd, IntAnd, IntCarry,
    IntEqual, IntLeftShift, IntLess, IntLessEqual, IntNegate, IntNotEqual, IntOr, IntRightShift,
    IntSExt, IntSignedBorrow, IntSignedCarry, IntSignedLess, IntSignedLessEqual, IntSub, IntXor,
    IntZExt, Load, PopCount, Return, Store,
};
use crate::space::SpaceManager;
use std::fmt::{Display, Formatter};

pub struct PcodeOperationDisplay<'a, T: SpaceManager> {
    pub(crate) op: PcodeOperation,
    pub(crate) spaces: &'a T,
}

impl<'a, T> Display for PcodeOperationDisplay<'a, T>
where
    T: SpaceManager,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.op {
            Copy { input, output } => {
                write!(
                    f,
                    "{} = {}",
                    output.display(self.spaces)?,
                    input.display(self.spaces)?
                )
            }
            PopCount { input, output } => {
                write!(
                    f,
                    "{} = popcount({})",
                    output.display(self.spaces)?,
                    input.display(self.spaces)?
                )
            }
            IntZExt { input, output } => {
                write!(
                    f,
                    "{} = zext({})",
                    output.display(self.spaces)?,
                    input.display(self.spaces)?
                )
            }
            IntSExt { input, output } => {
                write!(
                    f,
                    "{} = sext({})",
                    output.display(self.spaces)?,
                    input.display(self.spaces)?
                )
            }
            Store { output, input } => {
                write!(
                    f,
                    "{} = {}",
                    output.display(self.spaces)?,
                    input.display(self.spaces)?
                )
            }
            Load { input, output } => {
                write!(
                    f,
                    "{} = {}",
                    output.display(self.spaces)?,
                    input.display(self.spaces)?
                )
            }
            IntCarry {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = carry({}, {})",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?
            ),
            IntSignedCarry {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = s.carry({}, {})",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?
            ),
            IntSignedBorrow {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = s.borrow({}, {})",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?
            ),
            Int2Comp { output, input } => write!(
                f,
                "{} = -{}",
                output.display(self.spaces)?,
                input.display(self.spaces)?
            ),
            IntAdd {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} + {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            IntSub {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} - {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            IntAnd {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} & {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            IntOr {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} v {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            IntXor {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} ^ {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            IntRightShift {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} >> {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            IntLeftShift {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} << {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            IntLess {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} < {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            IntLessEqual {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} <= {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            IntSignedLess {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} s< {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            IntSignedLessEqual {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} s<= {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            IntEqual {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} == {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            IntNotEqual {
                output,
                input0,
                input1,
            } => write!(
                f,
                "{} = {} != {}",
                output.display(self.spaces)?,
                input0.display(self.spaces)?,
                input1.display(self.spaces)?,
            ),
            CallOther { output, inputs } => {
                if let Some(output) = output {
                    write!(f, "{} = ", output.display(self.spaces)?)?;
                }
                write!(f, "userop(")?;
                let mut args = Vec::with_capacity(inputs.len());
                for i in inputs {
                    args.push(format!("{}", i.display(self.spaces)?));
                }
                write!(f, "{}", args.join(", "))?;
                write!(f, ")")
            }
            CallInd { input } => write!(f, "call [{}]", input.display(self.spaces)?),
            Return { input } => write!(f, "return [{}]", input.display(self.spaces)?),
            Branch { input } => write!(f, "branch {}", input.display(self.spaces)?),
            CBranch { input0, input1 } => write!(
                f,
                "if {} branch {}",
                input1.display(self.spaces)?,
                input0.display(self.spaces)?
            ),
            BranchInd { input } => write!(f, "branch [{}]", input.display(self.spaces)?),
            Call { input } => write!(f, "call {}", input.display(self.spaces)?),
            IntNegate { input, output } => write!(
                f,
                "{} =  ~{}",
                output.display(self.spaces)?,
                input.display(self.spaces)?
            ),
            _ => write!(f, "<please impl print> {:?}", self.op),
        }
    }
}
