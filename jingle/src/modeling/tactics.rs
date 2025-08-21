use std::ops::{Deref, DerefMut};
use z3::{Solver, Tactic};

pub struct TacticSolver(Solver);

impl Deref for TacticSolver {
    type Target = Solver;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TacticSolver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Solver> for TacticSolver {
    fn from(solver: Solver) -> Self {
        Self(solver)
    }
}

impl TacticSolver {
    pub fn new() -> Self {
        let t = default_tactic();
        Self(t.solver())
    }
}

impl Default for TacticSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TacticSolver {
    fn clone(&self) -> Self {
        let new = default_tactic().solver();
        for x in &self.get_assertions() {
            new.assert(x);
        }
        Self(new)
    }
}

/// This tactic has been written with the goal of reducing the solve times
/// of the sorts of formulas we produce. Speedups of up to 100x have been observed
/// using it.
///
/// The rationale behind it is that we first simplify and eliminate variables, before eliminating
/// array expressions (in favor of UFs), and then eliminate UFs with ackermann reduction. The result
/// is an (often much simpler) pure bitvector problem allowing z3 to use a specialized solver.
fn default_tactic() -> Tactic {
    macro_rules! tactic {
        ($name:literal) => {
            Tactic::new($name)
        };
    }
    let simplify = tactic!("simplify");
    let solve_eqs = tactic!("solve-eqs");
    let rep = Tactic::repeat(&simplify.and_then(&solve_eqs), u32::MAX);
    let bvarray2uf = tactic!("bvarray2uf");
    let ackermannize_bv = tactic!("ackermannize_bv");
    let smt = tactic!("smt");

    rep.and_then(&bvarray2uf)
        .and_then(&ackermannize_bv)
        .and_then(&smt)
}
