use crate::analysis::cpa::lattice::{JoinSemiLattice, PartialJoinSemiLattice};
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
impl<C: PartialJoinSemiLattice> PartialOrd for SimpleLattice<C> {
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
