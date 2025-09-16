use crate::analysis::cpa::lattice::flat::FlatLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::iter::{empty, once};

pub type PcodeAddressLattice = FlatLattice<ConcretePcodeAddress>;

impl AbstractState for PcodeAddressLattice {
    type SuccessorIter = Box<dyn Iterator<Item = Self>>;

    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer<B: Borrow<PcodeOperation>>(&self, op: B) -> Box<dyn Iterator<Item = Self>> {
        let op = op.borrow();
        match &self {
            PcodeAddressLattice::Value(a) => match op {
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
            PcodeAddressLattice::Top => Box::new(once(PcodeAddressLattice::Top)),
        }
    }
}
