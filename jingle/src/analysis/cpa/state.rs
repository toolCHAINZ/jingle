use crate::analysis::cpa::lattice::JoinSemiLattice;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MergeOutcome {
    NoOp,
    Merged,
}

impl MergeOutcome {
    pub fn merged(&self) -> bool {
        matches!(self, MergeOutcome::Merged)
    }
}

pub struct Successor<'a, T>(Box<dyn Iterator<Item = T> + 'a>);

impl<'a, T: 'a> Successor<'a, T> {
    pub fn into_iter(mut self) -> impl Iterator<Item = T> + 'a {
        self.0
    }
}

impl<'a, T, I: Iterator<Item = T> + 'a> From<I> for Successor<'a, T> {
    fn from(value: I) -> Self {
        Self(Box::new(value))
    }
}

pub trait AbstractState: JoinSemiLattice + Clone {
    /// Determines how two abstract states should be merged. Rather than consuming states
    /// and returning a new state, we mutate the first state argument. In the context of
    /// CPA, the first state should be the state from the _reached_ list, NOT the new/merged
    /// state.
    ///
    /// Implementations of this function must ENSURE (the compiler can't help here) that
    /// the mutated State is
    /// [Greater](std::cmp::Ordering::Greater) or [Equal](std::cmp::Ordering::Equal)
    /// than it was going in. Violating this is a logic error that may make CPA not terminate.
    fn merge(&mut self, other: &Self) -> MergeOutcome;

    /// A naive "cartesian" merge of two states, replacing `existing_state` with their
    /// Least Upper Bound.
    fn merge_join(&mut self, new_state: &Self) -> MergeOutcome {
        if self == new_state {
            MergeOutcome::NoOp
        } else {
            self.join(new_state);
            MergeOutcome::Merged
        }
    }

    /// A naive "separate" merge of two states, simply duplicating `existing_state`.
    fn merge_sep(&mut self, _: &Self) -> MergeOutcome {
        MergeOutcome::NoOp
    }

    /// Determines whether the given abstract state is covered by the set of reached
    /// abstract states. If it is not covered, this returns [false], indicating that this
    /// state should be added to CPA's waitlist. Otherwise, nothing is done, effectively terminating
    /// the current "branch" of analysis.
    ///
    /// Determining whether a state "covers" another may be done piecewise or by combining reached
    /// states, and is usually done with respect to the [JoinSemiLattice] defined over abstract states.
    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool;

    /// A naive implementation of [stop] which checks for state covering in a piecewise manner.
    fn stop_sep<'a, T: Iterator<Item = &'a Self>>(&'a self, mut states: T) -> bool {
        states.any(|s| self <= s)
    }

    /// Given a pcode operation, returns an iterator of successor states.
    /// Decided to make this an iterator to allow making the state structures simpler
    /// (e.g. a resolved indirect jump could return an iterator of locations instead of
    /// having a special "the location is one in this list" variant
    fn transfer<B: Borrow<PcodeOperation>>(&self, opcode: B) -> Successor<Self>;
}
