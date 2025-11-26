use z3::ast::Bool;

use crate::{analysis::cfg::PcodeCfg, modeling::machine::MachineState};

enum CtlQuantifier {
    Existential,
    Universal,
}

enum CtlFormula {
    Bottom,
    Top,
    Proposition(Box<dyn Fn(MachineState) -> Bool>),
    Negation(Box<CtlFormula>),
    Conjunction(Box<CtlFormula>, Box<CtlFormula>),
    Disjunction(Box<CtlFormula>, Box<CtlFormula>),
    Implies(Box<CtlFormula>, Box<CtlFormula>),
    Iff(Box<CtlFormula>, Box<CtlFormula>),
    Path {
        quantifier: CtlQuantifier,
        path_formula: PathFormula,
    },
}

enum PathFormula {
    Next(Box<CtlFormula>),
    Eventually(Box<CtlFormula>),
    Always(Box<CtlFormula>),
    Until(Box<CtlFormula>, Box<CtlFormula>),
}

impl CtlFormula{
    pub fn check(g: PcodeCfg){
        let model = g.
    }
}