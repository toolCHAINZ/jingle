#![expect(non_snake_case)]

use crate::analysis::cfg::{CfgState, CfgStateModel, ModelTransition, PcodeCfgVisitor};
use std::borrow::Borrow;
use std::fmt;
use std::ops::{BitAnd, BitOr, Deref};
use std::rc::Rc;
use z3::Solver;
use z3::ast::Bool;

#[derive(Debug, Clone, Copy)]
pub enum CtlQuantifier {
    Existential,
    Universal,
}

type CtlPropClosure<M, D> = Rc<dyn Fn(&PcodeCfgVisitor<M, D>, Option<&D>) -> Bool + 'static>;
#[derive(Clone)]
pub struct CtlProp<N: CfgState, D: ModelTransition<N::Model>> {
    closure: CtlPropClosure<N, D>,
}

impl<T, N: CfgState, D: ModelTransition<N::Model>> From<T> for CtlProp<N, D>
where
    T: Fn(&PcodeCfgVisitor<N, D>, Option<&D>) -> Bool + 'static,
{
    fn from(value: T) -> Self {
        Self {
            closure: Rc::new(value),
        }
    }
}

impl<N: CfgState, D: ModelTransition<N::Model>> Deref for CtlProp<N, D> {
    type Target = CtlPropClosure<N, D>;
    fn deref(&self) -> &Self::Target {
        &self.closure
    }
}

#[derive(Clone)]
pub struct CtlUnary<N: CfgState, D: ModelTransition<N::Model>> {
    term: Rc<CtlFormula<N, D>>,
}

impl<N: CfgState, D: ModelTransition<N::Model>> From<CtlFormula<N, D>> for CtlUnary<N, D> {
    fn from(value: CtlFormula<N, D>) -> Self {
        Self {
            term: Rc::new(value),
        }
    }
}

impl<N: CfgState, D: ModelTransition<N::Model>> Deref for CtlUnary<N, D> {
    type Target = CtlFormula<N, D>;
    fn deref(&self) -> &Self::Target {
        self.term.as_ref()
    }
}

#[derive(Clone)]
pub struct CtlBinary<N: CfgState, D: ModelTransition<N::Model>> {
    pub left: Rc<CtlFormula<N, D>>,
    pub right: Rc<CtlFormula<N, D>>,
}

impl<N: CfgState, D: ModelTransition<N::Model>> From<(CtlFormula<N, D>, CtlFormula<N, D>)>
    for CtlBinary<N, D>
{
    fn from(a: (CtlFormula<N, D>, CtlFormula<N, D>)) -> Self {
        let (left, right) = a;
        Self {
            left: Rc::new(left),
            right: Rc::new(right),
        }
    }
}
#[derive(Clone)]
pub enum CtlFormula<N: CfgState, D: ModelTransition<N::Model>> {
    Bottom,
    Top,
    Proposition(CtlProp<N, D>),
    Negation(CtlUnary<N, D>),
    Conjunction(CtlBinary<N, D>),
    Disjunction(CtlBinary<N, D>),
    Implies(CtlBinary<N, D>),
    Iff(CtlBinary<N, D>),
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
    pub fn proposition<T: Into<CtlProp<N, D>>>(f: T) -> Self {
        CtlFormula::Proposition(f.into())
    }

    pub fn not(&self) -> Self {
        CtlFormula::Negation(self.clone().into())
    }

    pub fn and(&self, b: CtlFormula<N, D>) -> Self {
        CtlFormula::Conjunction((self.clone(), b).into())
    }

    pub fn or(&self, b: CtlFormula<N, D>) -> Self {
        CtlFormula::Disjunction((self.clone(), b).into())
    }

    pub fn implies(&self, b: CtlFormula<N, D>) -> Self {
        CtlFormula::Implies((self.clone(), b).into())
    }

    pub fn iff(&self, b: CtlFormula<N, D>) -> Self {
        CtlFormula::Iff((self.clone(), b).into())
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
    ///
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
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
impl<N: CfgState, D: ModelTransition<N::Model>> CtlFormula<N, D> {
    pub fn check(&self, g: &PcodeCfgVisitor<N, D>, solver: &Solver) -> Bool {
        
        match self {
            CtlFormula::Bottom => Bool::from_bool(false),
            CtlFormula::Top => Bool::from_bool(true),
            CtlFormula::Proposition(closure) => closure(g, g.transition()),
            CtlFormula::Negation(a) => a.check(g, solver).not(),
            CtlFormula::Conjunction(CtlBinary { left, right }) => {
                let l = left.check(g, solver);
                let r = right.check(g, solver);
                l.bitand(r)
            }
            CtlFormula::Disjunction(CtlBinary { left, right }) => {
                let l = left.check(g, solver);
                let r = right.check(g, solver);
                l.bitor(r)
            }
            CtlFormula::Implies(CtlBinary { left, right }) => {
                let l = left.check(g, solver);
                let r = right.check(g, solver);
                l.implies(&r)
            }
            CtlFormula::Iff(CtlBinary { left, right }) => {
                let left = left.check(g, solver);
                let right = right.check(g, solver);
                left.eq(&right)
            }
            CtlFormula::Path(PathFormula {
                operation: PathOperation::Next(inner),
                quantifier,
            }) => match quantifier {
                CtlQuantifier::Existential => inner.check_next_exists(g, solver),
                CtlQuantifier::Universal => inner.check_next_universal(g, solver),
            },
            CtlFormula::Path(path_formula) => {
                // rewritten formula guaranteed to only have state assertions
                // and next operations
                let rewrite = path_formula.rewrite();
                rewrite.check(g, solver)
            }
        }
    }

    pub(crate) fn check_next_exists(&self, g: &PcodeCfgVisitor<N, D>, solver: &Solver) -> Bool {
        let state = g.state().unwrap();
        let connect: Vec<_> = g
            .successors()
            .map(|a| {
                let successor = a.state().unwrap();
                let after = g.transition().unwrap().transition(state).unwrap();
                
                after.location_eq(successor)
            })
            .collect();
        let connect = Bool::or(&connect);
        let bools: Vec<_> = g
            .successors()
            .flat_map(|a| {
                let successor = a.state().unwrap();
                let check = self.check(&a, solver);
                let after = g.transition().unwrap().transition(state).unwrap();
                let imp = after
                    .location_eq(successor)
                    .implies(after.state_eq(successor));
                Some(check.bitand(imp))
            })
            .collect();
        Bool::or(&bools).bitand(connect)
    }

    pub(crate) fn check_next_universal(&self, g: &PcodeCfgVisitor<N, D>, solver: &Solver) -> Bool {
        let state = g.state().unwrap();
        let connect: Vec<_> = g
            .successors()
            .map(|a| {
                let successor = a.state().unwrap();
                let after = g.transition().unwrap().transition(state).unwrap();
                
                after.location_eq(successor)
            })
            .collect();
        let connect = Bool::or(&connect);
        let bools: Vec<_> = g
            .successors()
            .flat_map(|a| {
                let successor = a.state().unwrap();
                let check = self.check(&a, solver);
                let after = g.transition().unwrap().transition(state).unwrap();
                let imp = after
                    .location_eq(successor)
                    .implies(after.state_eq(successor));
                Some(check.bitand(imp))
            })
            .collect();
        Bool::and(&bools).bitand(connect)
    }
}
