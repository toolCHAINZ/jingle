use crate::analysis::cpa::lattice::{JoinSemiLattice, PartialJoinSemiLattice};
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::analysis::pcode_store::PcodeStore;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum SimpleLattice<C> {
    Value(C),
    Top,
}

impl<C> From<C> for SimpleLattice<C> {
    fn from(value: C) -> Self {
        Self::Value(value)
    }
}

impl<C> SimpleLattice<C> {
    pub fn is_top(&self) -> bool {
        matches!(self, Self::Top)
    }

    pub fn value(&self) -> Option<&C> {
        match self {
            Self::Top => None,
            Self::Value(c) => Some(c),
        }
    }
}
impl<C: crate::analysis::cpa::lattice::PartialJoinSemiLattice> PartialOrd for SimpleLattice<C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Top, Self::Top) => Some(Ordering::Equal),
            (Self::Top, _) => Some(Ordering::Greater),
            (_, Self::Top) => Some(Ordering::Less),
            (Self::Value(a), Self::Value(b)) => a.partial_cmp(b),
        }
    }
}

impl<C: PartialJoinSemiLattice> JoinSemiLattice for SimpleLattice<C> {
    fn join(&mut self, other: &Self) {
        match (&self, &other) {
            (Self::Top, _) => (),
            (_, Self::Top) => *self = Self::Top,
            (Self::Value(a), Self::Value(b)) => match a.partial_join(b) {
                None => *self = Self::Top,
                Some(c) => *self = Self::Value(c),
            },
        }
    }
}

impl<S: AbstractState + PartialJoinSemiLattice> AbstractState for SimpleLattice<S> {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        match (self, other) {
            (Self::Value(a), Self::Value(b)) => a.merge(b),
            _ => MergeOutcome::NoOp,
        }
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        match self {
            Self::Value(a) => a.stop(states.flat_map(|t| t.value())),
            Self::Top => true,
        }
    }

    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, opcode: B) -> Successor<'a, Self> {
        match self {
            SimpleLattice::Value(a) => a
                .transfer(opcode)
                .into_iter()
                .map(|a| SimpleLattice::Value(a))
                .into(),
            SimpleLattice::Top => std::iter::empty().into(),
        }
    }
}

impl<S: LocationState + AbstractState + PartialJoinSemiLattice> LocationState for SimpleLattice<S> {
    fn get_operation<T: PcodeStore>(&self, t: &T) -> Option<PcodeOperation> {
        match self {
            SimpleLattice::Value(a) => a.get_operation(t),
            SimpleLattice::Top => None,
        }
    }
}
