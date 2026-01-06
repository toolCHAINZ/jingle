use crate::analysis::cpa::lattice::flat::FlatLattice;
use crate::analysis::cpa::lattice::flat::FlatLattice::Value;
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::analysis::pcode_store::PcodeStore;
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

impl LocationState for PcodeAddressLattice {
    fn get_operation<T: PcodeStore>(&self, t: &T) -> Option<PcodeOperation> {
        match self {
            PcodeAddressLattice::Value(a) => t.get_pcode_op_at(a),
            PcodeAddressLattice::Top => None,
        }
    }
}

// Implement Strengthen for PcodeAddressLattice to support compound analysis
impl crate::analysis::compound::Strengthen<crate::analysis::stack_offset::StackOffsetState> for PcodeAddressLattice {}

impl crate::analysis::compound::Strengthen<crate::analysis::direct_valuation::DirectValuationState> for PcodeAddressLattice {}

// Implement Strengthen for PcodeAddressLattice against CompoundState<StackOffsetState, DirectValuationState>
impl crate::analysis::compound::Strengthen<
    crate::analysis::compound::CompoundState<
        crate::analysis::stack_offset::StackOffsetState,
        crate::analysis::direct_valuation::DirectValuationState
    >
> for PcodeAddressLattice {}

