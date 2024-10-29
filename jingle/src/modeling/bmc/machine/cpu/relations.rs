use crate::modeling::bmc::machine::cpu::{ConcretePcodeAddress, SymbolicPcodeAddress};
use crate::modeling::bmc::machine::memory::MemoryState;
use crate::JingleError;
use jingle_sleigh::PcodeOperation;
use z3::ast::{Ast, BV};
use z3::Context;

impl<'ctx> SymbolicPcodeAddress<'ctx> {
    pub(crate) fn apply_op(
        &self,
        memory: &MemoryState<'_, 'ctx, '_>,
        op: &PcodeOperation,
        z3: &'ctx Context,
    ) -> Result<Self, JingleError> {
        match op {
            PcodeOperation::Branch { input } => {
                Ok(ConcretePcodeAddress::from(input.offset).symbolize(z3))
            }
            PcodeOperation::CBranch { input0, input1 } => {
                let fallthrough = self.increment_pcode();
                let dest = ConcretePcodeAddress::from(input0).symbolize(z3);
                let take_branch =
                    memory
                        .read(input1)?
                        ._eq(&BV::from_u64(z3, 1, (input1.size * 8) as u32));
                let final_dest = take_branch.ite(&dest.0, &fallthrough.0);
                Ok(SymbolicPcodeAddress(final_dest))
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
