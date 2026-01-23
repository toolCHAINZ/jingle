use std::marker::PhantomData;

use jingle_sleigh::PcodeOperation;

use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::AbstractState;

/// A simple reducer that records every visited destination state in a `Vec`.
///
/// This reducer collects clones of destination states passed to `residue` in the
/// order they are observed by the CPA. When a `merged` event occurs, any earlier
/// recorded occurrences of the `dest_state` are replaced with clones of the
/// `merged_state`, so the collected history reflects merges performed by the CPA.
///
/// Note: replacement equality is determined by comparing `Debug` output of
/// states. This avoids adding extra trait bounds on `S` (such as `PartialEq`),
/// at the cost of relying on the stability/uniqueness of the `Debug` output for
/// the concrete state types used.
pub struct VecReducer<S>
where
    S: AbstractState,
{
    /// Collected visited states (destinations passed to `residue`).
    pub visited: Vec<S>,
    _phantom: PhantomData<S>,
}

impl<S> VecReducer<S>
where
    S: AbstractState,
{
    /// Create an empty `VecReducer` with reserved capacity.
    pub fn new_with_capacity(cap: usize) -> Self {
        Self {
            visited: Vec::with_capacity(cap),
            _phantom: Default::default(),
        }
    }
}

impl<S> Default for VecReducer<S>
where
    S: AbstractState,
{
    fn default() -> Self {
        Self {
            visited: Vec::new(),
            _phantom: Default::default(),
        }
    }
}

impl<S> Residue<S> for VecReducer<S>
where
    S: AbstractState,
{
    type Output = Vec<S>;

    /// Record the destination state into the internal `Vec`.
    ///
    /// The reducer stores clones of the `dest_state` argument in the order they
    /// are observed by the CPA.
    fn new_state(
        &mut self,
        _state: &S,
        dest_state: &S,
        _op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        self.visited.push(dest_state.clone());
    }

    /// When two abstract states are merged, replace earlier occurrences of
    /// `dest_state` in the recorded `visited` list with clones of `merged_state`.
    ///
    /// Replacement is performed by comparing `Debug` representations of entries
    /// to the `dest_state` Debug representation. This avoids additional trait
    /// bounds on `S` (such as `PartialEq`), at the cost of relying on Debug.
    fn merged_state(
        &mut self,
        _curr_state: &S,
        dest_state: &S,
        merged_state: &S,
        _op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        for entry in &mut self.visited {
            if entry == dest_state {
                *entry = merged_state.clone();
            }
        }
    }

    fn new() -> Self {
        Self::default()
    }

    /// Return the collected visited states.
    fn finalize(self) -> Self::Output {
        self.visited
    }
}
