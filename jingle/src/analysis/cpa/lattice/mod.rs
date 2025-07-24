pub mod simple;
pub mod pcode;
pub mod flat;

pub trait JoinSemiLattice: Eq + PartialOrd {
    fn join(&mut self, other: &Self);
    // todo: add top method?
    // fn is_top(&self) -> bool;
}

/// This trait exists to allow defining types that are
/// "lattice-y" without requiring that they specify a Top element.
/// Types implementing this trait can be used to construct
/// an actual Lattice using [super::simple_lattice::SimpleLattice]
pub trait PartialJoinSemiLattice: Eq + PartialOrd + Sized {
    fn partial_join(&self, other: &Self) -> Option<Self>;
}

impl<S1, S2> JoinSemiLattice for (S1, S2)
where
    S1: JoinSemiLattice,
    S2: JoinSemiLattice,
{
    fn join(&mut self, other: &Self) {
        self.0.join(&other.0);
        self.1.join(&other.1);
    }
}
