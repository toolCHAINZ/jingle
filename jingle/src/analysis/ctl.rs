use crate::analysis::cfg::{CfgState, ModelTransition, PcodeCfgVisitor};
use crate::{JingleError, analysis::cfg::PcodeCfg, modeling::machine::MachineState};
use std::borrow::Borrow;
use std::ops::{BitAnd, BitOr};
use z3::Solver;
use z3::ast::{Ast, Bool};
use z3_sys::AstKind::Quantifier;
use z3_sys::SortKind::Bool;

#[derive(Debug, Clone, Copy)]
pub enum CtlQuantifier {
    Existential,
    Universal,
}

#[derive(Clone)]
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

impl<N: CfgState, D: ModelTransition<<N as CfgState>::Model>> BitAnd for CtlFormula<N, D> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.and(rhs)
    }
}

impl<N: CfgState, D: ModelTransition<<N as CfgState>::Model>> BitOr for CtlFormula<N, D> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.or(rhs)
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

    pub fn and<U: Borrow<CtlFormula<N, D>>>(&self, b: U) -> Self {
        CtlFormula::Conjunction(Box::new(self.clone()), Box::new(b.borrow().clone()))
    }

    pub fn or<U: Borrow<CtlFormula<N, D>>>(&self, b: U) -> Self {
        CtlFormula::Disjunction(Box::new(self.clone()), Box::new(b.borrow().clone()))
    }

    pub fn implies<U: AsRef<CtlFormula<N, D>>>(&self, b: U) -> Self {
        CtlFormula::Implies(Box::new(self.clone()), Box::new(b.as_ref().clone()))
    }

    pub fn iff<U: AsRef<CtlFormula<N, D>>>(&self, b: U) -> Self {
        CtlFormula::Iff(Box::new(self.clone()), Box::new(b.as_ref().clone()))
    }

    pub fn path<T: Borrow<PathOperation<N, D>>>(quantifier: CtlQuantifier, pf: T) -> Self {
        CtlFormula::Path(PathFormula {
            quantifier,
            operation: pf.borrow().clone(),
        })
    }
}

pub fn AX<N: CfgState, D: ModelTransition<N::Model>, R: Borrow<CtlFormula<N, D>>>(
    a: R,
) -> CtlFormula<N, D> {
    CtlFormula::path(
        CtlQuantifier::Universal,
        PathOperation::next(a.borrow().clone()),
    )
}
pub fn EX<N: CfgState, D: ModelTransition<N::Model>, R: Borrow<CtlFormula<N, D>>>(
    a: R,
) -> CtlFormula<N, D> {
    CtlFormula::path(
        CtlQuantifier::Existential,
        PathOperation::next(a.borrow().clone()),
    )
}
pub fn AF<N: CfgState, D: ModelTransition<N::Model>, R: Borrow<CtlFormula<N, D>>>(
    a: R,
) -> CtlFormula<N, D> {
    CtlFormula::path(
        CtlQuantifier::Universal,
        PathOperation::eventually(a.borrow().clone()),
    )
}
pub fn EF<N: CfgState, D: ModelTransition<N::Model>, R: Borrow<CtlFormula<N, D>>>(
    a: R,
) -> CtlFormula<N, D> {
    CtlFormula::path(
        CtlQuantifier::Existential,
        PathOperation::eventually(a.borrow().clone()),
    )
}
pub fn AG<N: CfgState, D: ModelTransition<N::Model>, R: Borrow<CtlFormula<N, D>>>(
    a: R,
) -> CtlFormula<N, D> {
    CtlFormula::path(
        CtlQuantifier::Universal,
        PathOperation::always(a.borrow().clone()),
    )
}
pub fn EG<N: CfgState, D: ModelTransition<N::Model>, R: Borrow<CtlFormula<N, D>>>(
    a: R,
) -> CtlFormula<N, D> {
    CtlFormula::path(
        CtlQuantifier::Existential,
        PathOperation::always(a.borrow().clone()),
    )
}
pub fn AU<
    N: CfgState,
    D: ModelTransition<N::Model>,
    RA: Borrow<CtlFormula<N, D>>,
    RB: Borrow<CtlFormula<N, D>>,
>(
    a: RA,
    b: RB,
) -> CtlFormula<N, D> {
    CtlFormula::path(
        CtlQuantifier::Universal,
        PathOperation::until(a.borrow().clone(), b.borrow().clone()),
    )
}
pub fn EU<
    N: CfgState,
    D: ModelTransition<N::Model>,
    RA: Borrow<CtlFormula<N, D>>,
    RB: Borrow<CtlFormula<N, D>>,
>(
    a: RA,
    b: RB,
) -> CtlFormula<N, D> {
    CtlFormula::path(
        CtlQuantifier::Existential,
        PathOperation::until(a.borrow().clone(), b.borrow().clone()),
    )
}

#[derive(Debug, Clone)]
pub struct PathFormula<N: CfgState, D: ModelTransition<N::Model>> {
    quantifier: CtlQuantifier,
    operation: PathOperation<N, D>,
}

impl<N: CfgState, D: ModelTransition<N::Model>> PathFormula<N, D> {
    /// Rewrites certain CTL formulas into equivalent forms

    /// For example, A G φ can be rewritten as φ ∧ A X A G φ
    /// This is used to break down formulas when model checking
    ///
    /// After using this, resulting formulas will have a new term (what was inside the path operation)
    /// to evaluate on the current state a logical connective, and a path operator to apply to successors
    ///
    /// This allows for recursively unwinding CTL formulae over the CFG until there are no more successors
    /// (at which point the residual path operator is a no-op).
    fn rewrite(&self) -> CtlFormula<N, D> {
        match (self.quantifier, self.operation.clone()) {
            (CtlQuantifier::Universal, PathOperation::Always(phi)) => phi.and(AX(AG(phi.as_ref()))),
            (CtlQuantifier::Existential, PathOperation::Always(phi)) => {
                phi.and(EX(EG(phi.as_ref())))
            }
            (CtlQuantifier::Universal, PathOperation::Eventually(phi)) => {
                phi.or(AX(AF(phi.as_ref())))
            }
            (CtlQuantifier::Existential, PathOperation::Eventually(phi)) => {
                phi.or(EX(EF(phi.as_ref())))
            }
            (CtlQuantifier::Universal, PathOperation::Until(phi, psi)) => {
                psi.or(phi.and(AX(AU(phi.as_ref(), psi.as_ref()))))
            }
            (CtlQuantifier::Existential, PathOperation::Until(phi, psi)) => {
                psi.or(phi.and(EX(EU(phi.as_ref(), psi.as_ref()))))
            }
            _ => CtlFormula::Path(self.clone()),
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
            CtlFormula::Path(PathFormula {
                operation: PathOperation::Next(inner),
                quantifier,
            }) => match quantifier {
                CtlQuantifier::Existential => inner.check_next_exists(g, solver)?,
                CtlQuantifier::Universal => inner.check_next_universal(g, solver)?,
            },
            CtlFormula::Path(path_formula) => {
                // rewritten formula guaranteed to only have state assertions
                // and next operations
                let rewrite = path_formula.rewrite();
                rewrite.check(g, solver)?
            }
        };
        let id = g.location().model_id();
        let track = Bool::fresh_const(&id);
        solver.assert_and_track(val.clone(), &track);
        Ok(val.simplify())
    }

    pub(crate) fn check_next_exists(
        &self,
        g: &PcodeCfgVisitor<N, D>,
        solver: &Solver,
    ) -> Result<Bool, JingleError> {
        let bools: Vec<_> = g
            .successors()
            .flat_map(|a| {
                let check = self.check(&a, solver).ok()?;
                let simp = check.simplify();
                if matches!(simp.as_bool(), Some(false)) {
                    None
                } else {
                    Some(simp)
                }
            })
            .collect();
        Ok(Bool::or(&bools))
    }

    pub(crate) fn check_next_universal(
        &self,
        g: &PcodeCfgVisitor<N, D>,
        solver: &Solver,
    ) -> Result<Bool, JingleError> {
        let bools: Vec<_> = g
            .successors()
            .flat_map(|a| {
                let check = self.check(&a, solver).ok()?;
                Some(check)
            })
            .collect();
        Ok(Bool::and(&bools))
    }
}
