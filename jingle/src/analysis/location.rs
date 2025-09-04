use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::lattice::flat::FlatLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::iter::{empty, once};

#[derive(Clone, Debug, PartialOrd, PartialEq, Eq)]
#[expect(unused)]
pub struct SimpleLocation(FlatLattice<ConcretePcodeAddress>);

impl JoinSemiLattice for SimpleLocation {
    fn join(&mut self, other: &Self) {
        self.0.join(&other.0)
    }
}

impl From<ConcretePcodeAddress> for SimpleLocation {
    fn from(value: ConcretePcodeAddress) -> Self {
        Self(FlatLattice::Value(value))
    }
}

impl From<SMTLocation> for SimpleLocation {
    fn from(value: SMTLocation) -> Self {
        Self(value.0)
    }
}

impl AbstractState for SimpleLocation {
    type SuccessorIter = Box<dyn Iterator<Item = Self>>;

    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer(&self, op: &PcodeOperation) -> Box<dyn Iterator<Item = Self>> {
        match &self.0 {
            FlatLattice::Value(a) => match op {
                PcodeOperation::Branch { input } => {
                    Box::new(once(ConcretePcodeAddress::from(input.offset).into()))
                }
                PcodeOperation::CBranch { input0, .. } => {
                    let dest = ConcretePcodeAddress::resolve_from_varnode(input0, *a);
                    let fallthrough = a.next_pcode();
                    Box::new(once(dest.into()).chain(once(fallthrough.into())))
                }
                PcodeOperation::Call { .. } | PcodeOperation::CallOther { .. } => {
                    Box::new(once(a.next_pcode().into()))
                }
                PcodeOperation::Return { .. }
                | PcodeOperation::CallInd { .. }
                | PcodeOperation::BranchInd { .. } => Box::new(empty()),
                _ => Box::new(once(a.next_pcode().into())),
            },
            FlatLattice::Top => Box::new(once(Self(FlatLattice::Top))),
        }
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Eq)]
#[expect(unsued)]
pub struct SMTLocation(pub(crate) FlatLattice<ConcretePcodeAddress>);

impl JoinSemiLattice for SMTLocation {
    fn join(&mut self, other: &Self) {
        self.0.join(&other.0)
    }
}
