use crate::modeling::machine::cpu::concrete::{ConcretePcodeAddress, PcodeOffset};
use crate::modeling::machine::cpu::symbolic::SymbolicPcodeAddress;
use z3::ast::Ast;
use z3::{SatResult, Solver};

pub struct SymbolicAddressConcretization<'ctx> {
    solver: Solver<'ctx>,
    addr: SymbolicPcodeAddress<'ctx>,
}

impl<'ctx> SymbolicAddressConcretization<'ctx> {
    pub fn new_with_solver(solver: &Solver<'ctx>, addr: &SymbolicPcodeAddress<'ctx>) -> Self {
        Self {
            solver: solver.clone(),
            addr: addr.clone(),
        }
    }

    pub fn new(addr: &SymbolicPcodeAddress<'ctx>) -> Self {
        Self {
            solver: Solver::new(addr.pcode.get_ctx()),
            addr: addr.clone(),
        }
    }
}
impl Iterator for SymbolicAddressConcretization<'_> {
    type Item = ConcretePcodeAddress;

    fn next(&mut self) -> Option<Self::Item> {
        match self.solver.check() {
            SatResult::Unsat => None,
            SatResult::Unknown => None,
            SatResult::Sat => {
                let model = self.solver.get_model()?;
                let pcode = model.eval(&self.addr.pcode, true)?.as_u64()?;
                let machine = model.eval(&self.addr.machine, true)?.as_u64()?;
                let concrete = ConcretePcodeAddress {
                    pcode: pcode as PcodeOffset,
                    machine,
                };
                let diff_pcode = self
                    .addr
                    ._eq(&concrete.symbolize(self.solver.get_context()))
                    .not();
                self.solver.assert(&diff_pcode);
                Some(concrete)
            }
        }
    }
}
