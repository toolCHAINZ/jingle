use crate::modeling::machine::cpu::symbolic::SymbolicPcodeAddress;
use crate::modeling::machine::memory::MemoryState;
use crate::JingleError;
use jingle_sleigh::PcodeOperation;
use z3::ast::{Ast, BV};
use z3::Context;

impl<'ctx> SymbolicPcodeAddress<'ctx> {
    pub(crate) fn apply_op(
        &self,
        memory: &MemoryState<'ctx>,
        op: &PcodeOperation,
        z3: &'ctx Context,
    ) -> Result<Self, JingleError> {
        match op {
            PcodeOperation::Branch { input } | PcodeOperation::Call { input } => {
                Ok(self.interpret_branch_dest_varnode(input))
            }
            PcodeOperation::CBranch { input0, input1 } => {
                let fallthrough = self.increment_pcode();
                let dest = self.interpret_branch_dest_varnode(input0);
                let take_branch =
                    memory
                        .read(input1)?
                        ._eq(&BV::from_u64(z3, 1, (input1.size * 8) as u32));
                let machine = take_branch.ite(&dest.machine, &fallthrough.machine);
                let pcode = take_branch.ite(&dest.pcode, &fallthrough.pcode);
                Ok(SymbolicPcodeAddress { machine, pcode })
            }
            PcodeOperation::BranchInd { input }
            | PcodeOperation::CallInd { input }
            | PcodeOperation::Return { input } => {
                let dest = memory.read(input)?;
                SymbolicPcodeAddress::try_from_symbolic_dest(z3, &dest)
            }
            _ => Ok(self.increment_pcode()),
        }
    }
}
