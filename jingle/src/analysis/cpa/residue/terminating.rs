use std::marker::PhantomData;

use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::AbstractState;

/// A reducer that collects terminating states: states that never transition to other states.
///
/// A state is considered terminating if it has no successors over the transfer function
/// is applied.
///
/// The reducer tracks all states visited by the CPA and identifies which ones
/// never appear as source states in a state transition. When a merge occurs,
/// the merged state replaces the original destination state in the tracking logic.
pub struct TerminatingReducer<S>
where
    S: AbstractState,
{
    /// All destination states observed during the analysis.
    all_states: Vec<S>,
    /// States that have been observed as source states (i.e., have successors).
    non_terminating_states: Vec<S>,
    _phantom: PhantomData<S>,
}

impl<S> TerminatingReducer<S>
where
    S: AbstractState,
{
    /// Create an empty `TerminatingReducer`.
    pub fn new_with_capacity(cap: usize) -> Self {
        Self {
            all_states: Vec::with_capacity(cap),
            non_terminating_states: Vec::with_capacity(cap),
            _phantom: Default::default(),
        }
    }

    /// Compute the terminating states from the tracked information.
    ///
    /// A state is terminating if it appears in `all_states` but not in `non_terminating_states`.
    fn compute_terminating_states(self) -> Vec<S> {
        let mut terminating_states = Vec::new();
        for state in self.all_states {
            // A state is terminating if it's not in the non-terminating list
            if !self.non_terminating_states.iter().any(|s| s == &state) {
                terminating_states.push(state);
            }
        }
        terminating_states
    }
}

impl<S> Default for TerminatingReducer<S>
where
    S: AbstractState,
{
    fn default() -> Self {
        Self {
            all_states: Vec::new(),
            non_terminating_states: Vec::new(),
            _phantom: Default::default(),
        }
    }
}

impl<'a, S> Residue<'a, S> for TerminatingReducer<S>
where
    S: AbstractState,
{
    type Output = Vec<S>;

    /// Track a state transition from `state` to `dest_state`.
    ///
    /// The source `state` has at least one successor, so it is not terminating.
    /// The `dest_state` is recorded as a potential terminating state (to be verified later).
    fn new_state(
        &mut self,
        state: &S,
        dest_state: &S,
        _op: &Option<crate::analysis::pcode_store::PcodeOpRef<'a>>,
    ) {
        // The source state has successors, so it's not terminating
        if !self.non_terminating_states.iter().any(|s| s == state) {
            self.non_terminating_states.push(state.clone());
        }

        // Track the destination state
        if !self.all_states.iter().any(|s| s == dest_state) {
            self.all_states.push(dest_state.clone());
        }
    }

    /// Handle state merging by updating our tracking data.
    ///
    /// When states are merged, the current state is also a non-terminating state since
    /// it produced successors. We update references to the original destination
    /// state to point to the merged state.
    fn merged_state(
        &mut self,
        curr_state: &S,
        original_merged_state: &S,
        merged_state: &S,
        _op: &Option<crate::analysis::pcode_store::PcodeOpRef<'a>>,
    ) {
        // The current state has successors, so it's not terminating
        if !self.non_terminating_states.iter().any(|s| s == curr_state) {
            self.non_terminating_states.push(curr_state.clone());
        }

        // Replace the original merged state with the new merged state in our tracking
        for state in &mut self.all_states {
            if state == original_merged_state {
                *state = merged_state.clone();
            }
        }

        for state in &mut self.non_terminating_states {
            if state == original_merged_state {
                *state = merged_state.clone();
            }
        }
    }

    fn new() -> Self {
        Self::default()
    }

    /// Return the collected terminating states.
    ///
    /// Terminating states are those that were visited but never produced any successors.
    fn finalize(self) -> Self::Output {
        self.compute_terminating_states()
    }
}

/// Zero-sized factory for constructing `TerminatingReducer` instances.
///
/// Exported as a public zero-sized type so callers can pass the factory value
/// (or the `TERMINATING` const) to APIs like `with_residue`.
#[derive(Debug, Clone, Copy)]
pub struct TerminatingReducerFactory;

impl TerminatingReducerFactory {
    /// Create a new factory value (const-friendly).
    pub const fn new() -> Self {
        TerminatingReducerFactory
    }
}

impl Default for TerminatingReducerFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Ergonomic public constant that can be passed to `with_residue(...)`.
pub const TERMINATING: TerminatingReducerFactory = TerminatingReducerFactory;

/// Implement the reducer factory trait so this factory can be used by the CPA
/// wrapping mechanisms to instantiate `TerminatingReducer<A::State>`.
impl<A> crate::analysis::cpa::residue::ReducerFactoryForState<A> for TerminatingReducerFactory
where
    A: crate::analysis::cpa::ConfigurableProgramAnalysis,
    A::State: crate::analysis::cpa::state::AbstractState,
{
    type Reducer<'op> = TerminatingReducer<A::State>;

    fn make<'op>(&self) -> Self::Reducer<'op> {
        TerminatingReducer::default()
    }
}
