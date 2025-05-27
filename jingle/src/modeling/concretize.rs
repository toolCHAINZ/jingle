use crate::modeling::tactics::TacticSolver;
use std::fs;
use std::time::SystemTime;
use z3::ast::{Ast, BV, Bool};
use z3::{Context, Model, SatResult};

/// Implemented by types that represent expressions
/// that can be interpreted in terms of a symbolic state
pub trait Concretize<'ctx>: Clone {
    type Concretized;

    fn ctx(&self) -> &'ctx Context;

    fn eval(&self, model: &Model<'ctx>, model_completion: bool) -> Option<Self::Concretized>;

    fn make_counterexample(&self, c: &Self::Concretized) -> Bool<'ctx>;
}

impl<'ctx> Concretize<'ctx> for BV<'ctx> {
    type Concretized = u64;

    fn ctx(&self) -> &'ctx Context {
        self.get_ctx()
    }

    fn eval(&self, model: &Model<'ctx>, model_completion: bool) -> Option<Self::Concretized> {
        model.eval(self, model_completion)?.as_u64()
    }

    fn make_counterexample(&self, c: &Self::Concretized) -> Bool<'ctx> {
        let ctr = BV::from_u64(self.ctx(), *c, self.get_size());
        dbg!(self._eq(&ctr).not())
    }
}

impl<'ctx> Concretize<'ctx> for Bool<'ctx> {
    type Concretized = bool;

    fn ctx(&self) -> &'ctx Context {
        self.get_ctx()
    }

    fn eval(&self, model: &Model<'ctx>, model_completion: bool) -> Option<Self::Concretized> {
        model.eval(self, model_completion)?.as_bool()
    }

    fn make_counterexample(&self, c: &Self::Concretized) -> Bool<'ctx> {
        let ctr = Bool::from_bool(self.get_ctx(), *c);
        self._eq(&ctr).not()
    }
}

pub struct ConcretizationIterator<'ctx, T: Concretize<'ctx>> {
    val: T,
    assertions: Vec<Bool<'ctx>>,
}

impl<'ctx, T: Concretize<'ctx>> ConcretizationIterator<'ctx, T> {
    pub fn new_with_assertions<I: Iterator<Item = Bool<'ctx>>>(assertions: I, val: &T) -> Self {
        Self {
            assertions: assertions.collect(),
            val: val.clone(),
        }
    }

    pub fn new(val: &T) -> Self {
        Self {
            assertions: Default::default(),
            val: val.clone(),
        }
    }
}

impl<'ctx, T: Concretize<'ctx>> Iterator for ConcretizationIterator<'ctx, T> {
    type Item = T::Concretized;

    fn next(&mut self) -> Option<Self::Item> {
        let s = TacticSolver::new(self.val.ctx());
        for x in &self.assertions {
            s.assert(x);
        }
        let t = SystemTime::now();
        match s.check() {
            SatResult::Unsat => None,
            SatResult::Unknown => {
                let elapsed = t.elapsed().unwrap().as_nanos();
                fs::write(format!("formula/fail_{elapsed}.smt"), s.to_smt2()).unwrap();

                None
            }
            SatResult::Sat => {
                let elapsed = t.elapsed().unwrap().as_nanos();
                fs::write(format!("formula/test_{elapsed}.smt"), s.to_smt2()).unwrap();
                let model = s.get_model()?;
                let concrete = self.val.eval(&model, true)?;
                let diff = self.val.make_counterexample(&concrete);
                self.assertions.push(diff);
                Some(concrete)
            }
        }
    }
}
