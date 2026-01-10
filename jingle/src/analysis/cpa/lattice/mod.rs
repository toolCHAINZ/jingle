pub mod flat;
pub mod pcode;
pub mod simple;

use std::cmp::Ordering;

/// A thin abstraction over `PartialOrd` to decouple crate internals from the
/// standard trait. This mirrors the `PartialOrd` API.
pub trait AbstractPartialOrd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>;

    /// Convenience helpers mirroring `PartialOrd` default helper methods.
    fn lt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Less))
    }

    fn le(&self, other: &Self) -> bool {
        matches!(
            self.partial_cmp(other),
            Some(Ordering::Less) | Some(Ordering::Equal)
        )
    }

    fn gt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Greater))
    }

    fn ge(&self, other: &Self) -> bool {
        matches!(
            self.partial_cmp(other),
            Some(Ordering::Greater) | Some(Ordering::Equal)
        )
    }
}

/// Blanket impl so any existing `PartialOrd` types automatically implement
/// `AbstractPartialOrd`.
impl<T: PartialOrd> AbstractPartialOrd for T {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        PartialOrd::partial_cmp(self, other)
    }
}

pub trait JoinSemiLattice: Eq + AbstractPartialOrd {
    fn join(&mut self, other: &Self);
    // todo: add top method?
    // fn is_top(&self) -> bool;
}

/// This trait exists to allow defining types that are
/// "lattice-y" without requiring that they specify a Top element.
/// Types implementing this trait can be used to construct
/// an actual Lattice using [`SimpleLattice`](crate::analysis::cpa::lattice::simple::SimpleLattice)
pub trait PartialJoinSemiLattice: Eq + AbstractPartialOrd + Sized {
    fn partial_join(&self, other: &Self) -> Option<Self>;
}

impl<S1, S2> JoinSemiLattice for (S1, S2)
where
    S1: JoinSemiLattice + PartialOrd,
    S2: JoinSemiLattice + PartialOrd,
{
    fn join(&mut self, other: &Self) {
        self.0.join(&other.0);
        self.1.join(&other.1);
    }
}
