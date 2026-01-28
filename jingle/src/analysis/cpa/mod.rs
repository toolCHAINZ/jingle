pub mod final_reducer;
pub mod lattice;
pub mod reducer;
pub mod residue;
pub mod state;
pub mod vec_reducer;

pub use final_reducer::FinalReducer;
use tracing::{Level, span};
pub use vec_reducer::VecReducer;

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

    type Reducer: Residue<Self::State>;
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
    fn run_cpa<I: Borrow<Self::State>, P: PcodeStore>(
        &mut self,
        initial: I,
        pcode_store: &P,
    ) -> <<Self as ConfigurableProgramAnalysis>::Reducer as Residue<Self::State>>::Output {
        let initial = initial.borrow();
        let mut reducer = Self::Reducer::new();

        let mut waitlist: VecDeque<Self::State> = VecDeque::new();
        let mut reached: VecDeque<Self::State> = VecDeque::new();
        waitlist.push_front(initial.clone());
        reached.push_front(initial.clone());

        tracing::debug!("CPA started with initial state: {:?}", initial);
        tracing::debug!("Initial waitlist size: 1, reached size: 1");

        let mut iteration = 0;
        while let Some(state) = waitlist.pop_front() {
            let span = span!(Level::DEBUG, "cpa", iteration);
            let _enter = span.enter();
            iteration += 1;
            tracing::trace!("Processing state {:?}", state);
            tracing::trace!(
                "  Waitlist size: {}, Reached size: {}",
                waitlist.len(),
                reached.len()
            );

            let op = state.get_operation(pcode_store);
            tracing::trace!(
                "  Operation at state: {:?}",
                op.as_ref().map(|p| format!("{:x}", p.as_ref()))
            );

            let mut new_states = 0;
            let mut merged_states = 0;
            let mut stopped_states = 0;

            for dest_state in op
                .iter()
                .flat_map(|op| state.transfer(op.as_ref()).into_iter())
            {
                tracing::trace!("    Transfer produced dest_state: {}", dest_state);

                let mut was_merged = false;
                for reached_state in reached.iter_mut() {
                    let old_reached = reached_state.clone();
                    if reached_state.merge(&dest_state).merged() {
                        tracing::debug!("    Merged dest_state into existing reached_state");
                        tracing::debug!("      Merged state: {}", reached_state);
                        reducer.merged_state(&state, &old_reached, reached_state, &op);
                        waitlist.push_back(reached_state.clone());
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
                reducer.new_state(&state, &dest_state, &op);

                if !dest_state.stop(reached.iter()) {
                    tracing::trace!("    Adding new state to waitlist and reached");
                    waitlist.push_back(dest_state.clone());
                    reached.push_back(dest_state.clone());
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

        reducer.finalize()
    }

    fn with_residue<R: Residue<Self::State>>(self, r: R) -> ResidueWrapper<Self, R> {
        ResidueWrapper::wrap(self, r)
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
