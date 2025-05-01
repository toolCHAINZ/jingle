use std::fs;
use crate::modeling::machine::cpu::concrete::{ConcretePcodeAddress, PcodeOffset};
use crate::modeling::machine::cpu::symbolic::SymbolicPcodeAddress;
use z3::ast::{Ast, Bool};
use z3::{Params, SatResult, Solver};
use std::time::SystemTime;

pub struct SymbolicAddressConcretization<'ctx> {
    solver: Solver<'ctx>,
    addr: SymbolicPcodeAddress<'ctx>,
    assertions: Vec<Bool<'ctx>>,
}

impl<'ctx> SymbolicAddressConcretization<'ctx> {
    pub fn new_with_solver(solver: &Solver<'ctx>, addr: &SymbolicPcodeAddress<'ctx>) -> Self {
        Self {
            solver: solver.clone(),
            addr: addr.clone(),
            assertions: Vec::new(),
        }
    }

    pub fn new_with_assertions<T: Iterator<Item = Bool<'ctx>>>(
        assert: T,
        addr: &SymbolicPcodeAddress<'ctx>,
    ) -> Self {
        Self {
            solver: Solver::new_for_logic(addr.pcode.get_ctx(), "QF_ABV").unwrap(),
            assertions: assert.collect(),
            addr: addr.clone(),
        }
    }

    pub fn new(addr: &SymbolicPcodeAddress<'ctx>) -> Self {
        Self {
            solver: Solver::new(addr.pcode.get_ctx()),
            addr: addr.clone(),
            assertions: Vec::new(),
        }
    }
}
impl Iterator for SymbolicAddressConcretization<'_> {
    type Item = ConcretePcodeAddress;

    fn next(&mut self) -> Option<Self::Item> {
        let s = Solver::new_for_logic(self.solver.get_context(), "QF_ABV").unwrap();
        let mut p = Params::new(s.get_context());
        p.set_symbol("sat.phase", "always_false");
        p.set_u32("smt.threads", 8);
        p.set_bool("smt.ematching", false);
        s.set_params(&p);
        for x in &self.assertions {
            s.assert(x);
        }
        let t = SystemTime::now();
        match s.check() {
            SatResult::Unsat => None,
            SatResult::Unknown => {
                None
            }
            SatResult::Sat => {
                let elapsed = t.elapsed().unwrap().as_nanos();
                fs::write(format!("formula/test_{elapsed}.smt"), s.to_smt2()).unwrap();
                let model = s.get_model()?;
                let pcode = model.eval(&self.addr.pcode, true)?.as_u64()?;
                let machine = model.eval(&self.addr.machine, true)?.as_u64()?;
                let concrete = ConcretePcodeAddress {
                    pcode: pcode as PcodeOffset,
                    machine,
                };
                let diff_pcode = self.addr._eq(&concrete.symbolize(s.get_context())).not();
                self.assertions.push(diff_pcode);
                Some(concrete)
            }
        }
    }
}
