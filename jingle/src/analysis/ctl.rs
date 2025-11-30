use crate::JingleError;
use crate::analysis::cfg::{CfgState, ModelTransition, PcodeCfgVisitor};
use std::borrow::Borrow;
use std::fmt;
use std::ops::{BitAnd, BitOr};
use std::rc::Rc;
use z3::Solver;
use z3::ast::{Ast, Bool};

#[derive(Debug, Clone, Copy)]
pub enum CtlQuantifier {
    Existential,
    Universal,
}

#[derive(Clone)]
pub enum CtlFormula<N: CfgState, D: ModelTransition<N::Model>> {
    Bottom,
    Top,
    Proposition(Rc<dyn Fn(&N::Model, &D) -> Bool + 'static>),
    Negation(Rc<CtlFormula<N, D>>),
    Conjunction(Rc<CtlFormula<N, D>>, Rc<CtlFormula<N, D>>),
    Disjunction(Rc<CtlFormula<N, D>>, Rc<CtlFormula<N, D>>),
    Implies(Rc<CtlFormula<N, D>>, Rc<CtlFormula<N, D>>),
    Iff(Rc<CtlFormula<N, D>>, Rc<CtlFormula<N, D>>),
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
        CtlFormula::Proposition(Rc::new(f))
    }

    pub fn negation<T: AsRef<CtlFormula<N, D>>>(a: T) -> Self {
        CtlFormula::Negation(Rc::new(a.as_ref().clone()))
    }

    pub fn and<U: Borrow<CtlFormula<N, D>>>(&self, b: U) -> Self {
        CtlFormula::Conjunction(Rc::new(self.clone()), Rc::new(b.borrow().clone()))
    }

    pub fn or<U: Borrow<CtlFormula<N, D>>>(&self, b: U) -> Self {
        CtlFormula::Disjunction(Rc::new(self.clone()), Rc::new(b.borrow().clone()))
    }

    pub fn implies<U: AsRef<CtlFormula<N, D>>>(&self, b: U) -> Self {
        CtlFormula::Implies(Rc::new(self.clone()), Rc::new(b.as_ref().clone()))
    }

    pub fn iff<U: AsRef<CtlFormula<N, D>>>(&self, b: U) -> Self {
        CtlFormula::Iff(Rc::new(self.clone()), Rc::new(b.as_ref().clone()))
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

#[derive(Clone)]
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

#[derive(Clone)]
pub enum PathOperation<N: CfgState, D: ModelTransition<N::Model>> {
    Next(Rc<CtlFormula<N, D>>),
    Eventually(Rc<CtlFormula<N, D>>),
    Always(Rc<CtlFormula<N, D>>),
    Until(Rc<CtlFormula<N, D>>, Rc<CtlFormula<N, D>>),
}

impl<N: CfgState, D: ModelTransition<N::Model>> fmt::Debug for PathOperation<N, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathOperation::Next(inner) => write!(f, "Next({:?})", inner),
            PathOperation::Eventually(inner) => write!(f, "Eventually({:?})", inner),
            PathOperation::Always(inner) => write!(f, "Always({:?})", inner),
            PathOperation::Until(a, b) => write!(f, "Until({:?}, {:?})", a, b),
        }
    }
}

impl<N: CfgState, D: ModelTransition<N::Model>> fmt::Debug for PathFormula<N, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Path({:?}, {:?})", self.quantifier, self.operation)
    }
}

impl<N: CfgState, D: ModelTransition<N::Model>> PathOperation<N, D> {
    pub fn next<T: Borrow<CtlFormula<N, D>>>(f: T) -> Self {
        PathOperation::Next(Rc::new(f.borrow().clone()))
    }

    pub fn eventually<T: Borrow<CtlFormula<N, D>>>(f: T) -> Self {
        PathOperation::Eventually(Rc::new(f.borrow().clone()))
    }

    pub fn always<T: Borrow<CtlFormula<N, D>>>(f: T) -> Self {
        PathOperation::Always(Rc::new(f.borrow().clone()))
    }

    pub fn until<T: Borrow<CtlFormula<N, D>>, U: Borrow<CtlFormula<N, D>>>(a: T, b: U) -> Self {
        PathOperation::Until(Rc::new(a.borrow().clone()), Rc::new(b.borrow().clone()))
    }
}

impl<N: CfgState, D: ModelTransition<N::Model>> std::fmt::Debug for CtlFormula<N, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CtlFormula::Bottom => write!(f, "Bottom"),
            CtlFormula::Top => write!(f, "Top"),
            // Can't print the closure inside `Proposition`, use a placeholder instead.
            CtlFormula::Proposition(_) => write!(f, "Proposition({})", "proposition"),
            CtlFormula::Negation(a) => write!(f, "Negation({:?})", a),
            CtlFormula::Conjunction(l, r) => write!(f, "Conjunction({:?}, {:?})", l, r),
            CtlFormula::Disjunction(l, r) => write!(f, "Disjunction({:?}, {:?})", l, r),
            CtlFormula::Implies(l, r) => write!(f, "Implies({:?}, {:?})", l, r),
            CtlFormula::Iff(l, r) => write!(f, "Iff({:?}, {:?})", l, r),
            CtlFormula::Path(p) => write!(f, "Path({:?})", p),
        }
    }
}
impl<N: CfgState, D: ModelTransition<N::Model>> CtlFormula<N, D> {
    pub fn check(&self, g: &PcodeCfgVisitor<N, D>, solver: &Solver) -> Result<Bool, JingleError> {
        let val = match self {
            CtlFormula::Bottom => Bool::from_bool(false),
            CtlFormula::Top => Bool::from_bool(true),
            CtlFormula::Proposition(closure) => closure(
                g.state().ok_or(JingleError::ZeroSizedVarnode)?,
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
