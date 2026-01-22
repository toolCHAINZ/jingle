use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter, Result as FmtResult};

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

/// Iterator wrapper returned by `transfer` methods.
pub struct Successor<'a, T>(Box<dyn Iterator<Item = T> + 'a>);

impl<'a, T: 'a> IntoIterator for Successor<'a, T> {
    type Item = T;
    type IntoIter = Box<dyn Iterator<Item = T> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.0
    }
}

impl<'a, T, I: Iterator<Item = T> + 'a> From<I> for Successor<'a, T> {
    fn from(value: I) -> Self {
        Self(Box::new(value))
    }
}

/// Local trait for formatting abstract states.
///
/// We use this instead of `Display` on `AbstractState` to avoid orphan/coherence
/// issues (e.g., with generic tuple impls).
pub trait StateDisplay {
    fn fmt_state(&self, f: &mut Formatter<'_>) -> FmtResult;
}

/// Helper macro to implement `StateDisplay` for a concrete type by delegating to `Debug`.
macro_rules! impl_state_display_via_debug {
    ($ty:ty) => {
        impl StateDisplay for $ty {
            fn fmt_state(&self, f: &mut Formatter<'_>) -> FmtResult {
                write!(f, "{self:?}")
            }
        }
    };
}

/// Core trait for abstract states used by the CPA.
pub trait AbstractState: JoinSemiLattice + Clone + Debug + StateDisplay {
    /// Merge `other` into `self`. Mutate `self` and return whether merging occurred.
    /// The mutated `self` MUST be >= than it was before.
    fn merge(&mut self, other: &Self) -> MergeOutcome;

    /// Default cartesian merge using `join`.
    fn merge_join(&mut self, new_state: &Self) -> MergeOutcome {
        if self == new_state {
            MergeOutcome::NoOp
        } else {
            self.join(new_state);
            MergeOutcome::Merged
        }
    }

    /// Default separate merge (no-op).
    fn merge_sep(&mut self, _: &Self) -> MergeOutcome {
        MergeOutcome::NoOp
    }

    /// Stop predicate: is `self` covered by any of `states`?
    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool;

    /// Default stop predicate using piecewise ordering.
    fn stop_sep<'a, T: Iterator<Item = &'a Self>>(&'a self, mut states: T) -> bool {
        states.any(|s| {
            matches!(
                PartialOrd::partial_cmp(self, s),
                Some(Ordering::Less) | Some(Ordering::Equal)
            )
        })
    }

    /// Transfer function: return successor abstract states for a pcode operation.
    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, opcode: B) -> Successor<'a, Self>;
}

/// States that know their program location.
pub trait LocationState: AbstractState {
    fn get_operation<T: PcodeStore>(&self, t: &T) -> Option<PcodeOperation>;
    fn get_location(&self) -> Option<ConcretePcodeAddress>;
}

// Provide StateDisplay impls for known concrete state types by delegating to Debug.
// Only include impls for modules that are actually declared in the project.
impl_state_display_via_debug!(crate::analysis::back_edge::BackEdgeState);
impl_state_display_via_debug!(crate::analysis::bounded_branch::state::BoundedBranchState);
impl_state_display_via_debug!(crate::analysis::cpa::lattice::pcode::PcodeAddressLattice);
