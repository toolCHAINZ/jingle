use crate::pcode::branch::PcodeBranchDestination::{
    Branch, Conditional, IndirectBranch, IndirectCall, Return,
};
use crate::{IndirectVarNode, PcodeOperation, VarNode};

pub enum PcodeBranchDestination {
    Branch(VarNode),
    Call(VarNode),
    Conditional(VarNode),
    IndirectBranch(IndirectVarNode),
    IndirectCall(IndirectVarNode),
    Return(IndirectVarNode),
}
impl PcodeOperation {
    pub fn branch_destination(&self) -> Option<PcodeBranchDestination> {
        match self {
            PcodeOperation::Branch { input } | PcodeOperation::Call { input } => {
                Some(Branch(input.clone()))
            }
            PcodeOperation::CBranch { input0, .. } => Some(Conditional(input0.clone())),
            PcodeOperation::BranchInd { input } => Some(IndirectBranch(input.clone())),
            PcodeOperation::CallInd { input } => Some(IndirectCall(input.clone())),
            PcodeOperation::Return { input } => Some(Return(input.clone())),
            _ => None,
        }
    }
}
