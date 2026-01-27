use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::pcode_store::{PcodeOpRef, PcodeStore};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::ops::{Add, AddAssign};

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

impl Add for MergeOutcome {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::NoOp, Self::NoOp) => Self::NoOp,
            _ => Self::Merged,
        }
    }
}

impl AddAssign for MergeOutcome {
    fn add_assign(&mut self, rhs: Self) {
        (*self) = self.clone() + rhs
    }
}

/// Object-safe trait for cloneable iterators stored inside `DynIterBox`.
///
/// Any iterator type that is also `Clone` will implement this via the
/// blanket impl below; the `clone_box` method allows cloning through the
/// trait object so the boxed iterator can be cloned cheaply.
pub trait CloneableIter<'a, T>: Iterator<Item = T> {
    fn clone_box(&self) -> Box<dyn CloneableIter<'a, T> + 'a>;
}

impl<'a, T, I> CloneableIter<'a, T> for I
where
    I: Iterator<Item = T> + Clone + 'a,
{
    fn clone_box(&self) -> Box<dyn CloneableIter<'a, T> + 'a> {
        Box::new(self.clone())
    }
}

/// Newtype wrapper around a boxed cloneable iterator.
///
/// This provides `Clone` and `Iterator` implementations for the boxed trait
/// object, so upstream types (like `Successor::IntoIter`) can be `Clone`,
/// which is required by combinators such as `iproduct`.
pub struct DynIterBox<'a, T>(Box<dyn CloneableIter<'a, T> + 'a>);

impl<'a, T> DynIterBox<'a, T> {
    pub fn new<I>(iter: I) -> Self
    where
        I: Iterator<Item = T> + Clone + 'a,
    {
        DynIterBox(Box::new(iter))
    }
}

impl<'a, T> Clone for DynIterBox<'a, T> {
    fn clone(&self) -> Self {
        DynIterBox(self.0.clone_box())
    }
}

impl<'a, T> Iterator for DynIterBox<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.0.next()
    }

    // Forward size_hint if available
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<'a, T> Debug for DynIterBox<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // We can't show the inner iterator state meaningfully, so provide a simple marker.
        f.write_str("DynIterBox { .. }")
    }
}

/// Iterator wrapper returned by `transfer` methods.
///
/// This stores a boxed, cloneable iterator so the `Successor` itself can be
/// `Clone` without forcing collection of items into a `Vec`.
pub struct Successor<'a, T>(DynIterBox<'a, T>);

impl<'a, T> Successor<'a, T> {
    /// Create an empty successor iterator.
    pub fn empty() -> Self
    where
        T: 'a,
    {
        // std::iter::Empty implements Clone, so it satisfies CloneableIter via the blanket impl.
        Self(DynIterBox::new(std::iter::empty::<T>()))
    }
}

impl<'a, T> Clone for Successor<'a, T> {
    fn clone(&self) -> Self {
        Successor(self.0.clone())
    }
}

impl<'a, T: 'a> IntoIterator for Successor<'a, T> {
    type Item = T;
    type IntoIter = DynIterBox<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0
    }
}

/// Construct a `Successor` from any iterator that implements `Clone`.
///
/// We require the iterator to be `Clone` so that the boxed trait object can be
/// cloned cheaply. This avoids collecting into a `Vec`.
impl<'a, T: 'a, I> From<I> for Successor<'a, T>
where
    I: Iterator<Item = T> + Clone + 'a,
{
    fn from(value: I) -> Self {
        Self(DynIterBox::new(value))
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
    ///
    /// The returned `Successor` must be constructed from an iterator that
    /// implements `Clone`. This lets CPA clients clone the successor sequence
    /// cheaply when needed.
    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, opcode: B) -> Successor<'a, Self>;
}

/// States that know their program location.
pub trait LocationState: AbstractState {
    fn get_operation<'a, T: PcodeStore + ?Sized>(&'a self, t: &'a T) -> Option<PcodeOpRef<'a>>;
    fn get_location(&self) -> Option<ConcretePcodeAddress>;
}

// Provide StateDisplay impls for known concrete state types by delegating to Debug.
// Only include impls for modules that are actually declared in the project.
impl_state_display_via_debug!(crate::analysis::back_edge::BackEdgeState);
impl_state_display_via_debug!(crate::analysis::cpa::lattice::pcode::PcodeAddressLattice);
