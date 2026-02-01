use std::marker::PhantomData;

use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::AbstractState;

/// A reducer that collects "final" states—states that never transition to other states.
///
/// A state is considered final if it has no successors when its transfer function
/// is applied. These states represent terminal points in the analysis, such as:
/// - States that reach a return instruction
/// - States that reach an error condition
/// - States where the analysis naturally terminates
///
/// The reducer tracks all states visited by the CPA and identifies which ones
/// never appear as source states in a state transition. When a merge occurs,
/// the merged state replaces the original destination state in the tracking logic.
pub struct FinalReducer<S>
where
    S: AbstractState,
{
    /// All destination states observed during the analysis.
    all_states: Vec<S>,
    /// States that have been observed as source states (i.e., have successors).
    non_final_states: Vec<S>,
    _phantom: PhantomData<S>,
}

impl<S> FinalReducer<S>
where
    S: AbstractState,
{
    /// Create an empty `FinalReducer`.
    pub fn new_with_capacity(cap: usize) -> Self {
        Self {
            all_states: Vec::with_capacity(cap),
            non_final_states: Vec::with_capacity(cap),
            _phantom: Default::default(),
        }
    }

    /// Compute the final states from the tracked information.
    ///
    /// A state is final if it appears in `all_states` but not in `non_final_states`.
    fn compute_final_states(self) -> Vec<S> {
        let mut final_states = Vec::new();
        for state in self.all_states {
            // A state is final if it's not in the non-final list
            if !self.non_final_states.iter().any(|s| s == &state) {
                final_states.push(state);
            }
        }
        final_states
    }
}

impl<S> Default for FinalReducer<S>
where
    S: AbstractState,
{
    fn default() -> Self {
        Self {
            all_states: Vec::new(),
            non_final_states: Vec::new(),
            _phantom: Default::default(),
        }
    }
}

impl<'a, S> Residue<'a, S> for FinalReducer<S>
where
    S: AbstractState,
{
    type Output = Vec<S>;

    /// Track a state transition from `state` to `dest_state`.
    ///
    /// The source `state` has at least one successor, so it is not final.
    /// The `dest_state` is recorded as a potential final state (to be verified later).
    fn new_state(
        &mut self,
        state: &S,
        dest_state: &S,
        _op: &Option<crate::analysis::pcode_store::PcodeOpRef<'a>>,
    ) {
        // The source state has successors, so it's not final
        if !self.non_final_states.iter().any(|s| s == state) {
            self.non_final_states.push(state.clone());
        }

        // Track the destination state
        if !self.all_states.iter().any(|s| s == dest_state) {
            self.all_states.push(dest_state.clone());
        }
    }

    /// Handle state merging by updating our tracking data.
    ///
    /// When states are merged, the current state is also a non-final state since
    /// it produced successors. We update references to the original destination
    /// state to point to the merged state.
    fn merged_state(
        &mut self,
        curr_state: &S,
        original_merged_state: &S,
        merged_state: &S,
        _op: &Option<crate::analysis::pcode_store::PcodeOpRef<'a>>,
    ) {
        // The current state has successors, so it's not final
        if !self.non_final_states.iter().any(|s| s == curr_state) {
            self.non_final_states.push(curr_state.clone());
        }

        // Replace the original merged state with the new merged state in our tracking
        for state in &mut self.all_states {
            if state == original_merged_state {
                *state = merged_state.clone();
            }
        }

        for state in &mut self.non_final_states {
            if state == original_merged_state {
                *state = merged_state.clone();
            }
        }
    }

    fn new() -> Self {
        Self::default()
    }

    /// Return the collected final states.
    ///
    /// Final states are those that were visited but never produced any successors.
    fn finalize(self) -> Self::Output {
        self.compute_final_states()
    }
}
