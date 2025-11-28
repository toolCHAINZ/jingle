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

    pub fn conjunction<T: AsRef<CtlFormula<N, D>>, U: AsRef<CtlFormula<N, D>>>(a: T, b: U) -> Self {
        CtlFormula::Conjunction(Box::new(a.as_ref().clone()), Box::new(b.as_ref().clone()))
    }

    pub fn disjunction<T: AsRef<CtlFormula<N, D>>, U: AsRef<CtlFormula<N, D>>>(a: T, b: U) -> Self {
        CtlFormula::Disjunction(Box::new(a.as_ref().clone()), Box::new(b.as_ref().clone()))
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
}

macro_rules! ctl {
    (!$a:expr) => {
        CtlFormula::negation($ctl!($a))
    };
    (($a:tt ^ $b:tt)) => {
        CtlFormula::conjunction(ctl!($a), ctl!($b))
    };
    (($a:tt v $b:tt)) => {
        CtlFormula::disjunction(ctl!($a), ctl!($b))
    };
    (($a:tt => $b:tt)) => {
        CtlFormula::implies(ctl!($a), ctl!($b))
    };
    (($a:tt <=> $b:tt)) => {
        CtlFormula::iff(ctl!($a), ctl!($b))
    };
    ((AX $a:tt)) => {
        CtlFormula::path(CtlQuantifier::Universal, PathOperation::next(ctl!($a)))
    };
    ((EX $a:tt)) => {
        CtlFormula::path(CtlQuantifier::Existential, PathOperation::next(ctl!($a)))
    };
    ((AF $a:tt)) => {
        CtlFormula::path(CtlQuantifier::Universal, PathOperation::eventually(ctl!($a)))
    };
    ((EF $a:tt)) => {
        CtlFormula::path(CtlQuantifier::Existential, PathOperation::eventually(ctl!($a)))
    };
    ((AG $a:tt)) => {
        CtlFormula::path(CtlQuantifier::Universal, PathOperation::always(ctl!($a)))
    };
    (EG $a:tt) => {
        CtlFormula::path(CtlQuantifier::Existential, PathOperation::always(ctl!($a)))
    };
    (A [ $a:tt U $b:tt ]) => {
        CtlFormula::path(CtlQuantifier::Universal, PathOperation::until(ctl!($a), ctl!($b)))
    };
    (E [ $a:tt U $b:tt ]) => {
        CtlFormula::path(CtlQuantifier::Existential, PathOperation::until(ctl!($a), ctl!($b)))
    };
    ($f: expr) => {
        $f
    };
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
                ctl!((phi.clone()) ^ (AX (AX phi)))
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

    pub fn eventually<T: AsRef<CtlFormula<N, D>>>(f: T) -> Self {
        PathOperation::Eventually(Box::new(f.as_ref().clone()))
    }

    pub fn always<T: AsRef<CtlFormula<N, D>>>(f: T) -> Self {
        PathOperation::Always(Box::new(f.as_ref().clone()))
    }

    pub fn until<T: AsRef<CtlFormula<N, D>>, U: AsRef<CtlFormula<N, D>>>(a: T, b: U) -> Self {
        PathOperation::Until(Box::new(a.as_ref().clone()), Box::new(b.as_ref().clone()))
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
