use crate::analysis::cpa::lattice::flat::FlatLattice;
use crate::analysis::cpa::lattice::flat::FlatLattice::Value;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::iter::once;

pub type PcodeAddressLattice = FlatLattice<ConcretePcodeAddress>;

impl AbstractState for PcodeAddressLattice {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, op: B) -> Successor<'a, Self> {
        let op = op.borrow();
        match &self {
            PcodeAddressLattice::Value(a) => a.transfer(op).into_iter().map(Value).into(),
            PcodeAddressLattice::Top => once(PcodeAddressLattice::Top).into(),
        }
    }
}
