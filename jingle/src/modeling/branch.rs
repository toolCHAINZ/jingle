use crate::error::JingleError;
use crate::modeling::ModelingContext;
use crate::modeling::branch::BlockEndBehavior::{Fallthrough, UnconditionalBranch};
use crate::sleigh::{GeneralizedVarNode, VarNode};
use serde::{Deserialize, Serialize};
use std::ops::Not;
use z3::ast::{Ast, BV};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockConditionalBranchInfo {
    pub condition: VarNode,
    pub destination: GeneralizedVarNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchConstraint {
    pub last: BlockEndBehavior,
    pub conditional_branches: Vec<BlockConditionalBranchInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlockEndBehavior {
    Fallthrough(VarNode),
    UnconditionalBranch(GeneralizedVarNode),
}

impl BlockEndBehavior {
    pub fn read_dest_metadata<'ctx, 'a, T: ModelingContext<'ctx>>(
        &self,
        ctx: &'a T,
    ) -> Result<BV<'ctx>, JingleError> {
        match self {
            Fallthrough(f) => Ok(BV::from_u64(ctx.get_jingle().z3, 0, (f.size * 8) as u32)),
            UnconditionalBranch(b) => {
                match b {
                    // Direct branch
                    GeneralizedVarNode::Direct(d) => ctx.get_final_state().read_varnode_metadata(d),
                    // Indirect branch, we want to only inspect the pointer
                    GeneralizedVarNode::Indirect(i) => ctx
                        .get_final_state()
                        .read_varnode_metadata(&i.pointer_location),
                }
            }
        }
    }
    pub fn read_dest<'ctx, 'a, T: ModelingContext<'ctx>>(
        &self,
        ctx: &'a T,
    ) -> Result<BV<'ctx>, JingleError> {
        match self {
            Fallthrough(f) => Ok(BV::from_u64(
                ctx.get_jingle().z3,
                f.offset,
                (f.size * 8) as u32,
            )),
            UnconditionalBranch(b) => {
                match b {
                    // Direct branch
                    GeneralizedVarNode::Direct(d) => Ok(BV::from_u64(
                        ctx.get_jingle().z3,
                        d.offset,
                        (d.size * 8) as u32,
                    )),
                    // Indirect branch, we want to only inspect the pointer
                    GeneralizedVarNode::Indirect(i) => ctx
                        .get_final_state()
                        .read_varnode(&i.pointer_location)
                        .map(|f| f.simplify()),
                }
            }
        }
    }
}

impl BranchConstraint {
    pub fn new(last: &VarNode) -> Self {
        Self {
            last: Fallthrough(last.clone()),
            conditional_branches: Default::default(),
        }
    }

    pub fn with_same_final_branch(other: &Self) -> Self {
        Self {
            conditional_branches: vec![],
            last: other.last.clone(),
        }
    }

    pub fn has_branch(&self) -> bool {
        match self.last {
            Fallthrough(_) => !self.conditional_branches.is_empty(),
            UnconditionalBranch(_) => true,
        }
    }
    pub fn push_conditional(&mut self, cond: &BlockConditionalBranchInfo) {
        self.conditional_branches.push(cond.clone());
    }

    pub fn set_last(&mut self, new_last: &GeneralizedVarNode) {
        self.last = UnconditionalBranch(new_last.clone())
    }

    pub fn build_bv<'ctx, 'a, T: ModelingContext<'ctx>>(
        &self,
        ctx: &'a T,
    ) -> Result<BV<'ctx>, JingleError> {
        let mut dest_bv = self.last.read_dest(ctx)?;
        for cond_branch in self.conditional_branches.iter().rev() {
            let condition_bv = ctx
                .get_final_state()
                .read_varnode(&cond_branch.condition)?
                ._eq(&BV::from_i64(
                    ctx.get_jingle().z3,
                    0,
                    (cond_branch.condition.size * 8) as u32,
                ))
                .not();
            let branch_dest = match &cond_branch.destination {
                GeneralizedVarNode::Direct(d) => {
                    BV::from_u64(ctx.get_jingle().z3, d.offset, (d.size * 8) as u32)
                }
                GeneralizedVarNode::Indirect(a) => ctx.get_final_state().read(a.into())?,
            };
            dest_bv = condition_bv.ite(&branch_dest, &dest_bv);
        }
        Ok(dest_bv)
    }

    pub fn build_bv_metadata<'ctx, 'a, T: ModelingContext<'ctx>>(
        &self,
        ctx: &'a T,
    ) -> Result<BV<'ctx>, JingleError> {
        let mut dest_bv = self.last.read_dest_metadata(ctx)?;
        for cond_branch in self.conditional_branches.iter().rev() {
            let condition_bv = ctx
                .get_final_state()
                .read_varnode(&cond_branch.condition)?
                ._eq(&BV::from_i64(
                    ctx.get_jingle().z3,
                    0,
                    (&cond_branch.condition.size * 8) as u32,
                ))
                .not();
            let branch_dest = ctx
                .get_final_state()
                .read_metadata(cond_branch.destination.clone())?;
            dest_bv = condition_bv.ite(&branch_dest, &dest_bv);
        }
        Ok(dest_bv)
    }
}
