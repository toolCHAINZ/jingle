use crate::modeling::tactics::TacticSolver;
use z3::ast::{BV, Bool};
use z3::{Model, SatResult};

/// Implemented by types that represent expressions
/// that can be interpreted in terms of a symbolic state
pub trait Concretize: Clone {
    type Concretized;

    fn eval(&self, model: &Model, model_completion: bool) -> Option<Self::Concretized>;

    fn make_counterexample(&self, c: &Self::Concretized) -> Bool;
}

impl Concretize for BV {
    type Concretized = u64;

    fn eval(&self, model: &Model, model_completion: bool) -> Option<Self::Concretized> {
        model.eval(self, model_completion)?.as_u64()
    }

    fn make_counterexample(&self, c: &Self::Concretized) -> Bool {
        let ctr = BV::from_u64(*c, self.get_size());
        self.eq(&ctr).not()
    }
}

impl Concretize for Bool {
    type Concretized = bool;

    fn eval(&self, model: &Model, model_completion: bool) -> Option<Self::Concretized> {
        model.eval(self, model_completion)?.as_bool()
    }

    fn make_counterexample(&self, c: &Self::Concretized) -> Bool {
        let ctr = Bool::from_bool(*c);
        self.eq(&ctr).not()
    }
}

pub struct ConcretizationIterator<T: Concretize> {
    val: T,
    assertions: Vec<Bool>,
}

impl<T: Concretize> ConcretizationIterator<T> {
    pub fn new_with_assertions<I: Iterator<Item = Bool>>(assertions: I, val: &T) -> Self {
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

impl<T: Concretize> Iterator for ConcretizationIterator<T> {
    type Item = T::Concretized;

    fn next(&mut self) -> Option<Self::Item> {
        let s = TacticSolver::new();
        for x in &self.assertions {
            s.assert(x);
        }
        match s.check() {
            SatResult::Unsat => None,
            SatResult::Unknown => None,
            SatResult::Sat => {
                let model = s.get_model()?;
                let concrete = self.val.eval(&model, true)?;
                let diff = self.val.make_counterexample(&concrete);
                self.assertions.push(diff);
                Some(concrete)
            }
        }
    }
}
