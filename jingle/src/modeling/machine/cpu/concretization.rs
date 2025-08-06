use crate::modeling::concretize::Concretize;
use crate::modeling::machine::cpu::concrete::{ConcretePcodeAddress, PcodeOffset};
use crate::modeling::machine::cpu::symbolic::SymbolicPcodeAddress;
use z3::ast::{Ast, Bool};
use z3::{Context, Model};

impl Concretize for SymbolicPcodeAddress {
    type Concretized = ConcretePcodeAddress;

    fn ctx(&self) -> & Context {
        self.machine.get_ctx()
    }

    fn eval(&self, model: &Model, model_completion: bool) -> Option<Self::Concretized> {
        let machine = model.eval(&self.machine, model_completion)?.as_u64()?;
        let pcode = model.eval(&self.pcode, model_completion)?.as_u64()?;
        Some(ConcretePcodeAddress {
            machine,
            pcode: pcode as PcodeOffset,
        })
    }

    fn make_counterexample(&self, c: &Self::Concretized) -> Bool {
        self._eq(&c.symbolize(self.ctx())).not()
    }
}
