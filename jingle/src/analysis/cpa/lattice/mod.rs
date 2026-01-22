
pub mod flat;
pub mod pcode;
// pub mod simple;

pub trait JoinSemiLattice: Eq + PartialOrd {
    fn join(&mut self, other: &Self);
    // todo: add top method?
    // fn is_top(&self) -> bool;
}

/// This trait exists to allow defining types that are
/// "lattice-y" without requiring that they specify a Top element.
/// Types implementing this trait can be used to construct
/// an actual Lattice using [`SimpleLattice`](crate::analysis::cpa::lattice::simple::SimpleLattice)
pub trait PartialJoinSemiLattice: Eq + PartialOrd + Sized {
    fn partial_join(&self, other: &Self) -> Option<Self>;
}
