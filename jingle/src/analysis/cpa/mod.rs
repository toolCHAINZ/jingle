pub mod lattice;
pub mod state;

use crate::analysis::cpa::state::{AbstractState, Successor};
use std::borrow::Borrow;
use std::collections::VecDeque;
use std::fmt::Debug;

/**
A trait representing Configurable Program Analysis, a tunable unified framework for
dataflow and model checking algorithms. This implementation is based on the presentation of
CPA contained in Chapter 16 of
[The Handbook of Model Checking](https://link.springer.com/book/10.1007/978-3-319-10575-8)

CPA operates on abstract states, which are required to form a Lattice, specifically a
[JoinSemiLattice] (which requires a [join](JoinSemiLattice::join) operation). CPA applies
concrete transitions to abstract states, producing more abstract states. These states can
be merged when control flow merges (potentially losing information) or kept separate, providing
a large degree of flexibility. The algorithm terminates when no reached abstract state produces any
unreached abstract state, indicating a fixed point over the given domain.
As the abstract states form a lattice, this algorithm is guaranteed to terminate on any finite
set of abstract states.
*/
pub trait ConfigurableProgramAnalysis {
    /// An abstract state. Usually (but not necessarily) represents a single program location.
    type State: AbstractState + Debug;

    /// Generates an iterator of successor states for a given abstract state. This represents the
    /// transition relation of CPA. While this trait makes no reference to this transition relation
    /// or the types involved in it, it is assumed that an implemented analysis will contain the
    /// data necessary to query the transition relation for successor states.
    fn successor_states<'a>(&self, state: &'a Self::State) -> Successor<'a, Self::State>;

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
    fn reduce(&mut self, _state: &Self::State, _dest_state: &Self::State) {}

    /// The CPA algorithm. Implementors should not need to customize this function.
    ///
    /// Returns an iterator over abstract states reached from the initial abstract state.
    fn run_cpa<I: Borrow<Self::State>>(&mut self, initial: I) -> impl Iterator<Item = Self::State> {
        let initial = initial.borrow();
        let mut waitlist: VecDeque<Self::State> = VecDeque::new();
        let mut reached: VecDeque<Self::State> = VecDeque::new();
        waitlist.push_front(initial.clone());
        reached.push_front(initial.clone());
        while let Some(state) = waitlist.pop_front() {
            for dest_state in self.successor_states(&state).into_iter() {
                self.reduce(&state, &dest_state);
                for reached_state in reached.iter_mut() {
                    if reached_state.merge(&dest_state).merged() {
                        waitlist.push_back(reached_state.clone());
                    }
                }

                if !dest_state.stop(reached.iter()) {
                    waitlist.push_back(dest_state.clone());
                    reached.push_back(dest_state.clone());
                }
            }
        }
        reached.into_iter()
    }
}
