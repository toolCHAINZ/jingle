use crate::pcode::branch::PcodeBranchDestination::{
    Branch, Call, Conditional, IndirectBranch, IndirectCall, Return,
};
use crate::{IndirectVarNode, PcodeOperation, VarNode};

pub enum PcodeBranchDestination {
    Branch(VarNode),
    Call(VarNode),
    Conditional(VarNode),
    IndirectBranch(IndirectVarNode),
    IndirectCall(IndirectVarNode),
    Return(IndirectVarNode),
    // todo: add CallOther?
}

impl PcodeBranchDestination {
    pub fn is_indirect(&self) -> bool {
        matches!(self, IndirectBranch(_) | IndirectCall(_) | Return(_))
    }
}
impl PcodeOperation {
    pub fn branch_destination(&self) -> Option<PcodeBranchDestination> {
        match self {
            PcodeOperation::Branch { input } => Some(Branch(input.clone())),
            PcodeOperation::Call { dest: input, .. } => Some(Call(input.clone())),
            PcodeOperation::CBranch { input0, .. } => Some(Conditional(input0.clone())),
            PcodeOperation::BranchInd { input } => Some(IndirectBranch(input.clone())),
            PcodeOperation::CallInd { input } => Some(IndirectCall(input.clone())),
            PcodeOperation::Return { input } => Some(Return(input.clone())),
            _ => None,
        }
    }
}
