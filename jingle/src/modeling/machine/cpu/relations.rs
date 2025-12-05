use crate::JingleError;
use crate::modeling::machine::cpu::symbolic::SymbolicPcodeAddress;
use crate::modeling::machine::memory::MemoryState;
use jingle_sleigh::PcodeOperation;

impl SymbolicPcodeAddress {
    pub(crate) fn apply_op(
        &self,
        memory: &MemoryState,
        op: &PcodeOperation,
    ) -> Result<Self, JingleError> {
        match op {
            PcodeOperation::Branch { input } | PcodeOperation::Call { dest: input, .. } => {
                Ok(self.interpret_branch_dest_varnode(input))
            }
            PcodeOperation::CBranch { input0, input1 } => {
                let fallthrough = self.increment_pcode();
                let dest = self.interpret_branch_dest_varnode(input0);
                let take_branch = memory.read(input1)?.extract(0, 0).eq(1);
                let machine = take_branch.ite(&dest.machine, &fallthrough.machine);
                let pcode = take_branch.ite(&dest.pcode, &fallthrough.pcode);
                Ok(SymbolicPcodeAddress { machine, pcode })
            }
            PcodeOperation::BranchInd { input }
            | PcodeOperation::CallInd { input }
            | PcodeOperation::Return { input } => {
                let dest = memory.read(input)?;
                SymbolicPcodeAddress::try_from_symbolic_dest(&dest)
            }
            _ => Ok(self.increment_pcode()),
        }
    }
}
