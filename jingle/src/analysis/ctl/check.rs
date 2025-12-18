use z3::{Model, Solver, ast::Bool};

use crate::{
    JingleError,
    analysis::{
        cfg::{CfgState, CfgStateModel, ModelTransition, PcodeCfgVisitor},
        ctl::CtlFormula,
    },
};

pub trait CtlModelCheck<N: CfgState, D: ModelTransition<N::Model>> {
    fn check_with_context(
        &self,
        formula: CtlFormula<N, D>,
        visitor: PcodeCfgVisitor<N, D>,
        solver: &Solver,
    ) -> Result<CheckResult, JingleError>;

    fn check(
        &self,
        formula: CtlFormula<N, D>,
        visitor: PcodeCfgVisitor<N, D>,
    ) -> Result<CheckResult, JingleError> {
        let solver = Solver::new();
        self.check_with_context(formula, visitor, &solver)
    }
}

pub enum CheckResult {
    Sat(Model),
    Unsat(Option<Bool>),
}

pub struct DepthFirstCheck;

impl<N: CfgState, D: ModelTransition<N::Model>> CtlModelCheck<N, D> for DepthFirstCheck {
    fn check_with_context(
        &self,
        f: CtlFormula<N, D>,
        visitor: PcodeCfgVisitor<N, D>,
        solver: &Solver,
    ) -> Result<CheckResult, JingleError> {
        for ele in visitor.successors() {
            solver.push();
            // guaranteed by construction to be Some if there are any successors
            let trans = visitor.transition().unwrap();
            let after = trans.transition(visitor.state().unwrap())?;
            solver.assert(after.state_eq(ele.state().unwrap()));
            solver.pop(1);
        }
        todo!()
    }
}
