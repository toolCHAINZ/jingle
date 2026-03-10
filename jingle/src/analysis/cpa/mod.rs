// Keep module declarations grouped at the top for readability.
pub mod lattice;
pub mod residue;
pub mod state;
// `vec_reducer` and the other reducer implementations have been moved into `residue/`.

// Re-export ergonomic factory constants and the moved `FinalReducer` from the `residue` module
pub use crate::analysis::cpa::residue::{CFG, TerminatingReducer, VEC};
use tracing::{Level, span};

use crate::analysis::cpa::residue::{Residue, ResidueWrapper};
use crate::analysis::cpa::state::{AbstractState, LocationState};
use crate::analysis::pcode_store::PcodeStore;
use std::borrow::Borrow;
use std::collections::VecDeque;
use std::fmt::Debug;

/**
A trait representing Configurable Program Analysis, a tunable unified framework for
dataflow and model checking algorithms. This implementation is based on the presentation of
CPA contained in Chapter 16 of
[The Handbook of Model Checking](https://link.springer.com/book/10.1007/978-3-319-10575-8)

CPA operates on abstract states, which are required to form a Lattice, specifically a
[`JoinSemiLattice`](crate::analysis::cpa::lattice::JoinSemiLattice) (which requires a
[`join`](crate::analysis::cpa::lattice::JoinSemiLattice::join) operation). CPA applies
concrete transitions to abstract states, producing more abstract states. These states can
be merged when control flow merges (potentially losing information) or kept separate, providing
a large degree of flexibility. The algorithm terminates when no reached abstract state produces any
unreached abstract state, indicating a fixed point over the given domain.
As the abstract states form a lattice, this algorithm is guaranteed to terminate on any finite
set of abstract states.
*/
pub trait ConfigurableProgramAnalysis: Sized {
    /// An abstract state.
    type State: AbstractState + Debug;

    // Reducer is a generic associated type parameterized by an op lifetime `'op`.
    // This allows reducers to be expressed that accept/store `PcodeOpRef<'op>`
    // without requiring clones.
    type Reducer<'op>: residue::Residue<'op, Self::State>;
}

/**
An extension trait for [`ConfigurableProgramAnalysis`] that provides the `run_cpa` algorithm.
This trait is only implemented for CPAs whose states are [`LocationState`]s, which enables
the standard CPA algorithm to retrieve operations from program locations.
*/
pub trait RunnableConfigurableProgramAnalysis: ConfigurableProgramAnalysis
where
    Self::State: LocationState,
{
    /// The CPA algorithm. Implementors should not need to customize this function.
    ///
    /// Returns an iterator over abstract states reached from the initial abstract state.
    ///
    /// The function is generic over an `'op` lifetime which is the lifetime of
    /// p-code operation references returned by the `PcodeStore` (i.e. the store's
    /// borrow lifetime). The reducer type is instantiated for that same `'op`
    /// lifetime so it can accept `PcodeOpRef<'op>` without cloning.
    fn run_cpa<'op, I: Borrow<Self::State>, P: PcodeStore<'op> + ?Sized>(
        &self,
        initial: I,
        pcode_store: &'op P,
    ) -> <<Self as ConfigurableProgramAnalysis>::Reducer<'op> as residue::Residue<'op, Self::State>>::Output
    where
        Self::State: 'op,
    {
        let initial = initial.borrow();
        // Construct the reducer specialized for the `'op` lifetime.
        let mut reducer = <Self::Reducer<'op> as residue::Residue<'op, Self::State>>::new();

        // Use index-based waitlist to eliminate clones.
        // `reached` only grows (never shrinks), so indices remain stable.
        let mut waitlist: VecDeque<usize> = VecDeque::new();
        let mut reached: Vec<Self::State> = vec![initial.clone()];
        waitlist.push_back(0);

        tracing::debug!("CPA started with initial state: {:?}", initial);
        tracing::debug!("Initial waitlist size: 1, reached size: 1");

        let mut iteration = 0;
        while let Some(state_idx) = waitlist.pop_front() {
            let span = span!(Level::DEBUG, "cpa", iteration);
            let _enter = span.enter();
            iteration += 1;

            let reached_len = reached.len();

            tracing::trace!("Processing state at index {}", state_idx);
            tracing::trace!(
                "  Waitlist size: {}, Reached size: {}",
                waitlist.len(),
                reached_len
            );

            let mut new_states = 0;
            let mut merged_states = 0;
            let mut stopped_states = 0;

            // Collect all (op, successor_state) transitions from the current state.
            // For forward analysis this fetches the op at the current location and
            // applies the transfer function. For backward analysis this looks up
            // predecessor nodes and applies the backward transfer, returning one
            // pair per predecessor.
            let transitions = reached[state_idx].get_transitions(pcode_store);

            for (op, dest_state) in transitions {
                tracing::trace!("    Transfer produced dest_state: {}", dest_state);
                let op_ref = Some(op);

                let mut was_merged = false;
                for (idx, reached_state) in reached.iter_mut().enumerate() {
                    if reached_state.merge(&dest_state).is_merged() {
                        tracing::debug!("    Merged dest_state into existing reached_state");
                        tracing::debug!("      Merged state: {}", reached_state);
                        // Call the reducer's merged_state with state indices and operation
                        reducer.merged_state(state_idx, idx, &op_ref);
                        waitlist.push_back(idx);
                        merged_states += 1;
                        was_merged = true;
                    }
                }

                // If we merged the destination into an existing reached state, we've already
                // enqueued the merged (reached) state, so skip further handling for this dest.
                if was_merged {
                    continue;
                }

                // Only record a new state in the reducer if it will actually be added to `reached`.
                // record that a new state was reached without merging
                tracing::debug!("Adding new state without merging: {}", dest_state);

                if !dest_state.stop(reached.iter()) {
                    tracing::trace!("    Adding new state to waitlist and reached");
                    // Push to reached first, then enqueue its index
                    let new_idx = reached.len();
                    reached.push(dest_state);
                    // Notify reducer of the transition using indices and operation
                    reducer.new_state(state_idx, new_idx, &op_ref);
                    waitlist.push_back(new_idx);
                    new_states += 1;
                } else {
                    tracing::trace!("    State stopped (already covered)");
                    stopped_states += 1;
                }
            }

            if new_states > 0 || merged_states > 0 || stopped_states > 0 {
                tracing::debug!(
                    "Iteration {} summary: {} new state(s), {} merge(s), {} stopped",
                    iteration,
                    new_states,
                    merged_states,
                    stopped_states
                );
            }
        }

        tracing::debug!(
            "CPA completed after {} iterations. Total states reached: {}",
            iteration,
            reached.len()
        );

        reducer.finalize(reached)
    }

    fn with_residue<F>(self, f: F) -> ResidueWrapper<Self, F>
    where
        for<'op> F: crate::analysis::cpa::residue::ReducerFactoryForState<Self>,
    {
        ResidueWrapper::wrap(self, f)
    }
}

pub trait IntoState<C: ConfigurableProgramAnalysis>: Sized {
    fn into_state(self, c: &C) -> C::State;
}

// Blanket implementation: any CPA with LocationState automatically gets RunnableConfigurableProgramAnalysis
impl<T> RunnableConfigurableProgramAnalysis for T
where
    T: ConfigurableProgramAnalysis,
    T::State: LocationState,
{
}
