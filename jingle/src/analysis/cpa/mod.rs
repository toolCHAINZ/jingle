pub mod lattice;
pub mod state;

use crate::analysis::cpa::state::{AbstractState, LocationState};
use crate::analysis::pcode_store::PcodeStore;
use jingle_sleigh::PcodeOperation;
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

    /// Allows for accumulating information about a program not specific to particular abstract
    /// states.
    ///
    /// The standard CPA algorithm only accumulates program information in abstract states.
    /// However, it is often convenient to collect global program information not represented in any
    /// one state. Examples include building a CFG for the program or identifying back-edges.
    /// This method allows for implementing types to explicitly state the side-effect they would
    /// like to have on their analysis without trying to shove it into the successor iterator.
    ///
    /// This method will be called for every visited transition in the CPA, before merging. So,
    /// for every pair of states A,B visited by the CPA where A => B, this function will be called
    /// with arguments (A, B).
    ///
    /// Note that this should be used with caution if a CPA has a non-sep Merge definition; states
    /// may be refined after the CPA has made some sound effect
    fn reduce(
        &mut self,
        _state: &Self::State,
        _dest_state: &Self::State,
        _op: &Option<PcodeOperation>,
    ) {
    }

    /// A hook for when two abstract states are merged.
    fn merged(
        &mut self,
        _curr_state: &Self::State,
        _dest_state: &Self::State,
        _merged_state: &Self::State,
        _op: &Option<PcodeOperation>,
    ) {
    }
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
    ) -> Vec<Self::State> {
        let initial = initial.borrow();
        let mut waitlist: VecDeque<Self::State> = VecDeque::new();
        let mut reached: VecDeque<Self::State> = VecDeque::new();
        waitlist.push_front(initial.clone());
        reached.push_front(initial.clone());

        tracing::debug!("CPA started with initial state: {:?}", initial);
        tracing::debug!("Initial waitlist size: 1, reached size: 1");

        let mut iteration = 0;
        while let Some(state) = waitlist.pop_front() {
            iteration += 1;
            tracing::trace!("Iteration {}: Processing state {:?}", iteration, state);
            tracing::trace!(
                "  Waitlist size: {}, Reached size: {}",
                waitlist.len(),
                reached.len()
            );

            let op = state.get_operation(pcode_store);
            tracing::trace!("  Operation at state: {:?}", op);

            let mut new_states = 0;
            let mut merged_states = 0;
            let mut stopped_states = 0;

            for dest_state in op.iter().flat_map(|op| state.transfer(op).into_iter()) {
                tracing::trace!("    Transfer produced dest_state: {:?}", dest_state);
                self.reduce(&state, &dest_state, &op);

                let mut was_merged = false;
                for reached_state in reached.iter_mut() {
                    if reached_state.merge(&dest_state).merged() {
                        tracing::trace!("    Merged dest_state into existing reached_state");
                        tracing::trace!("      Merged state: {:?}", reached_state);
                        self.merged(&state, &dest_state, reached_state, &op);
                        waitlist.push_back(reached_state.clone());
                        merged_states += 1;
                        was_merged = true;
                    }
                }

                if !dest_state.stop(reached.iter()) {
                    tracing::trace!("    Adding new state to waitlist and reached");
                    waitlist.push_back(dest_state.clone());
                    reached.push_back(dest_state.clone());
                    new_states += 1;
                } else if !was_merged {
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

        reached.into()
    }

    /// Convenience wrapper: construct an initial `State` using the CPA's `make_initial_state`
    /// helper and then run the CPA. Accepts any input that implements `IntoState<Self>`.
    fn run_cpa_from<I: IntoState<Self>, P: PcodeStore>(
        &mut self,
        initial: I,
        pcode_store: &P,
    ) -> Vec<Self::State> {
        let state = initial.into_state(self);
        // `Self::State` implements `Borrow<Self::State>` (Borrow is implemented for `T`),
        // so we can pass the owned state to `run_cpa`.
        self.run_cpa(state, pcode_store)
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
