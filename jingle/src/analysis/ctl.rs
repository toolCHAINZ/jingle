use crate::analysis::cfg::{CfgState, ModelTransition, PcodeCfgVisitor};
use crate::{JingleError, analysis::cfg::PcodeCfg, modeling::machine::MachineState};
use std::borrow::Borrow;
use std::ops::{BitAnd, BitOr};
use z3::Solver;
use z3::ast::{Ast, Bool};
use z3_sys::AstKind::Quantifier;

#[derive(Debug, Clone, Copy)]
pub enum CtlQuantifier {
    Existential,
    Universal,
}

#[derive(Debug, Clone)]
pub enum CtlFormula<N: CfgState, D: ModelTransition<N::Model>> {
    Bottom,
    Top,
    Proposition(Box<dyn Fn(&N::Model, &D) -> Bool>),
    Negation(Box<CtlFormula<N, D>>),
    Conjunction(Box<CtlFormula<N, D>>, Box<CtlFormula<N, D>>),
    Disjunction(Box<CtlFormula<N, D>>, Box<CtlFormula<N, D>>),
    Implies(Box<CtlFormula<N, D>>, Box<CtlFormula<N, D>>),
    Iff(Box<CtlFormula<N, D>>, Box<CtlFormula<N, D>>),
    Path(PathFormula<N, D>),
}

impl<N: CfgState,D: ModelTransition<<N as CfgState>::Model>> BitAnd for CtlFormula<N,D>{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        CtlFormula::conjunction(&self, &rhs)
    }
}

impl<N: CfgState,D: ModelTransition<<N as CfgState>::Model>> BitOr for CtlFormula<N,D>{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        CtlFormula::disjunction(&self, &rhs)
    }
}

impl<N: CfgState, D: ModelTransition<N::Model>> CtlFormula<N, D> {
    pub fn bottom() -> Self {
        CtlFormula::Bottom
    }
    pub fn top() -> Self {
        CtlFormula::Top
    }
    pub fn proposition(f: impl Fn(&N::Model, &D) -> Bool + 'static) -> Self {
        CtlFormula::Proposition(Box::new(f))
    }

    pub fn negation<T: AsRef<CtlFormula<N, D>>>(a: T) -> Self {
        CtlFormula::Negation(Box::new(a.as_ref().clone()))
    }

    pub fn conjunction<T: Borrow<CtlFormula<N, D>>, U: Borrow<CtlFormula<N, D>>>(a: T, b: U) -> Self {
        CtlFormula::Conjunction(Box::new(a.borrow().clone()), Box::new(b.borrow().clone()))
    }

    pub fn disjunction<T: Borrow<CtlFormula<N, D>>, U: Borrow<CtlFormula<N, D>>>(a: T, b: U) -> Self {
        CtlFormula::Disjunction(Box::new(a.borrow().clone()), Box::new(b.borrow().clone()))
    }

    pub fn implies<T: AsRef<CtlFormula<N, D>>, U: AsRef<CtlFormula<N, D>>>(a: T, b: U) -> Self {
        CtlFormula::Implies(Box::new(a.as_ref().clone()), Box::new(b.as_ref().clone()))
    }

    pub fn iff<T: AsRef<CtlFormula<N, D>>, U: AsRef<CtlFormula<N, D>>>(a: T, b: U) -> Self {
        CtlFormula::Iff(Box::new(a.as_ref().clone()), Box::new(b.as_ref().clone()))
    }

    pub fn path<T: Borrow<PathOperation<N, D>>>(quantifier: CtlQuantifier, pf: T) -> Self {
        CtlFormula::Path(PathFormula {
            quantifier,
            operation: pf.borrow().clone(),
        })
    }

    pub fn AX<T: Borrow<CtlFormula<N, D>>>(a: T) -> Self {
        Self::path(CtlQuantifier::Universal, PathOperation::next(a))
    }

    pub fn EX<T: Borrow<CtlFormula<N, D>>>(a: T) -> Self {
        Self::path(CtlQuantifier::Existential, PathOperation::next(a))
    }

    pub fn AF<T: Borrow<CtlFormula<N, D>>>(a: T) -> Self {
        Self::path(CtlQuantifier::Universal, PathOperation::eventually(a))
    }

    pub fn EF<T: Borrow<CtlFormula<N, D>>>(a: T) -> Self {
        Self::path(CtlQuantifier::Existential, PathOperation::eventually(a))
    }

    pub fn AG<T: Borrow<CtlFormula<N, D>>>(a: T) -> Self {
        Self::path(CtlQuantifier::Universal, PathOperation::always(a))
    }

    pub fn EG<T: Borrow<CtlFormula<N, D>>>(a: T) -> Self {
        Self::path(CtlQuantifier::Existential, PathOperation::always(a))
    }

    pub fn AU<T: Borrow<CtlFormula<N, D>>, U: Borrow<CtlFormula<N, D>>>(a: T, b: U) -> Self {
        Self::path(
            CtlQuantifier::Universal,
            PathOperation::until(a, b),
        )
    }

    pub fn EU<T: Borrow<CtlFormula<N, D>>, U: Borrow<CtlFormula<N, D>>>(a: T, b: U) -> Self {
        Self::path(
            CtlQuantifier::Existential,
            PathOperation::until(a, b),
        )
    }
}

#[derive(Debug, Clone)]
pub struct PathFormula<N: CfgState, D: ModelTransition<N::Model>> {
    quantifier: CtlQuantifier,
    operation: PathOperation<N, D>,
}

impl<N: CfgState, D: ModelTransition<N::Model>> PathFormula<N, D> {
    fn rewrite(&self) -> CtlFormula<N, D> {
        match (self.quantifier, self.operation.clone()) {
            (CtlQuantifier::Universal, PathOperation::Always(phi)) => {
                phi ^ AX(AG(phi))
            }
            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum PathOperation<N: CfgState, D: ModelTransition<N::Model>> {
    Next(Box<CtlFormula<N, D>>),
    Eventually(Box<CtlFormula<N, D>>),
    Always(Box<CtlFormula<N, D>>),
    Until(Box<CtlFormula<N, D>>, Box<CtlFormula<N, D>>),
}

impl<N: CfgState, D: ModelTransition<N::Model>> PathOperation<N, D> {
    pub fn next<T: Borrow<CtlFormula<N, D>>>(f: T) -> Self {
        PathOperation::Next(Box::new(f.borrow().clone()))
    }

    pub fn eventually<T: Borrow<CtlFormula<N, D>>>(f: T) -> Self {
        PathOperation::Eventually(Box::new(f.borrow().clone()))
    }

    pub fn always<T: Borrow<CtlFormula<N, D>>>(f: T) -> Self {
        PathOperation::Always(Box::new(f.borrow().clone()))
    }

    pub fn until<T: Borrow<CtlFormula<N, D>>, U: Borrow<CtlFormula<N, D>>>(a: T, b: U) -> Self {
        PathOperation::Until(Box::new(a.borrow().clone()), Box::new(b.borrow().clone()))
    }
}

impl<N: CfgState, D: ModelTransition<N::Model>> CtlFormula<N, D> {
    pub fn check(&self, g: &PcodeCfgVisitor<N, D>, solver: &Solver) -> Result<Bool, JingleError> {
        let val = match self {
            CtlFormula::Bottom => Bool::from_bool(false),
            CtlFormula::Top => Bool::from_bool(true),
            CtlFormula::Proposition(closure) => closure(
                g.state().ok_or(JingleError::EmptyBlock)?,
                g.transition().ok_or(JingleError::EmptyBlock)?,
            ),
            CtlFormula::Negation(a) => a.check(g, solver)?.not(),
            CtlFormula::Conjunction(left, right) => {
                let l = left.check(g, solver)?;
                let r = right.check(g, solver)?;
                l.bitand(r)
            }
            CtlFormula::Disjunction(left, right) => {
                let l = left.check(g, solver)?;
                let r = right.check(g, solver)?;
                l.bitor(r)
            }
            CtlFormula::Implies(left, right) => {
                let l = left.check(g, solver)?;
                let r = right.check(g, solver)?;
                l.implies(&r)
            }
            CtlFormula::Iff(l, r) => {
                let left = l.check(g, solver)?;
                let right = r.check(g, solver)?;
                left.eq(&right)
            }
            CtlFormula::Path(path_formula) => {
                // Use path_formula.quantifier and path_formula.operation
                match path_formula.quantifier {
                    CtlQuantifier::Existential => path_formula.operation.check_exists(g, solver)?,
                    CtlQuantifier::Universal => path_formula.operation.check_forall(g, solver)?,
                }
            }
        };
        let id = g.location().model_id();
        let track = Bool::fresh_const(&id);
        solver.assert_and_track(val.clone(), &track);
        Ok(val.simplify())
    }
}

impl<N: CfgState, D: ModelTransition<N::Model>> PathOperation<N, D> {
    fn check_exists(
        &self,
        g: &PcodeCfgVisitor<N, D>,
        solver: &Solver,
    ) -> Result<Bool, JingleError> {
        let bools: Vec<_> = match self {
            PathOperation::Next(formula) => g
                .successors()
                .map(|n| formula.check(g))
                .flat_map(|o| o.ok())
                .flat_map(|b| {
                    let simp = b.simplify();
                    if simp.as_bool() == Some(false) {
                        None
                    } else {
                        Some(b)
                    }
                })
                .collect(),
            PathOperation::Eventually(_) => {}
            PathOperation::Always(_) => {}
            PathOperation::Until(_, _) => {}
        };
    }
    fn check_forall(
        &self,
        g: &PcodeCfgVisitor<N, D>,
        solver: &Solver,
    ) -> Result<Bool, JingleError> {
        // Placeholder for universal path checking
        Err(JingleError::EmptyBlock)
    }
}
