use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter, LowerHex};
use std::hash::Hash;
use crate::analysis::cpa::lattice::JoinSemiLattice;

#[derive(PartialEq, Eq, Copy, Clone, Hash, Debug)]
pub enum FlatLattice<C> {
    Value(C),
    Top,
}

impl<C> From<C> for FlatLattice<C> {
    fn from(value: C) -> Self {
        FlatLattice::Value(value)
    }
}

impl<C: Display> Display for FlatLattice<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FlatLattice::Value(a) => f
                .debug_tuple("FlatLattice")
                .field(&format_args!("{}", a))
                .finish(),
            FlatLattice::Top => write!(f, "FlatLattice(Top)"),
        }
    }
}

impl<C: LowerHex> LowerHex for FlatLattice<C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FlatLattice::Value(a) => f
                .debug_tuple("FlatLattice")
                .field(&format_args!("{:x}", a))
                .finish(),
            FlatLattice::Top => write!(f, "FlatLattice(Top)"),
        }
    }
}

impl<C: PartialOrd + PartialEq + Clone> FlatLattice<C> {
    pub fn is_top(&self) -> bool {
        matches!(self, FlatLattice::Top)
    }

    pub fn value(&self) -> Option<&C> {
        match self {
            FlatLattice::Value(c) => Some(c),
            FlatLattice::Top => None,
        }
    }
}

impl<C: PartialOrd + PartialEq + Clone> PartialOrd for FlatLattice<C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (&self, &other) {
            (Self::Top, Self::Top) => Some(Ordering::Equal),
            (Self::Top, Self::Value(_)) => Some(Ordering::Greater),
            (Self::Value(_), Self::Top) => Some(Ordering::Less),
            (Self::Value(a), Self::Value(b)) => {
                if a == b {
                    Some(Ordering::Equal)
                } else {
                    None
                }
            }
        }
    }
}

impl<C: PartialOrd + Eq + Clone> JoinSemiLattice for FlatLattice<C> {
    fn join(&mut self, other: &Self) {
        match (&self, other) {
            (Self::Top, _) => *self = Self::Top,
            (_, Self::Top) => *self = Self::Top,
            (Self::Value(a), Self::Value(b)) => {
                if a == b {
                    // do nothing
                } else {
                    *self = Self::Top
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::analysis::cpa::lattice::flat::FlatLattice;

    #[test]
    pub fn test_flat_lattice() {
        let val1 = FlatLattice::Value(4u64);
        let val2 = FlatLattice::Value(5u64);
        let top = FlatLattice::Top;
        assert_eq!(val1, val1);
        assert_eq!(val2, val2);
        assert_eq!(top, top);
        assert_ne!(val1, val2);
        assert_ne!(val1, top);
        assert!(top > val1);
        assert!(top > val2);
        assert!(val1.partial_cmp(&val2).is_none());
        assert!(val2.partial_cmp(&val1).is_none());
    }
}
