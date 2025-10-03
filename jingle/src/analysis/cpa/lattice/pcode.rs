use crate::analysis::cpa::lattice::flat::FlatLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::iter::{empty, once};

pub type PcodeAddressLattice = FlatLattice<ConcretePcodeAddress>;

impl AbstractState for PcodeAddressLattice {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer<B: Borrow<PcodeOperation>>(&self, op: B) -> Successor<Self> {
        let op = op.borrow();
        match &self {
            PcodeAddressLattice::Value(a) => match op {
                PcodeOperation::Branch { input } => {
                    once(ConcretePcodeAddress::from(input.offset).into()).into()
                }
                PcodeOperation::CBranch { input0, .. } => {
                    let dest = ConcretePcodeAddress::resolve_from_varnode(input0, *a);
                    let fallthrough = a.next_pcode();
                    once(dest.into()).chain(once(fallthrough.into())).into()
                }
                PcodeOperation::Call { .. } | PcodeOperation::CallOther { .. } => {
                    once(a.next_pcode().into()).into()
                }
                PcodeOperation::Return { .. }
                | PcodeOperation::CallInd { .. }
                | PcodeOperation::BranchInd { .. } => empty().into(),
                _ => once(a.next_pcode().into()).into(),
            },
            PcodeAddressLattice::Top => once(PcodeAddressLattice::Top).into(),
        }
    }
}
