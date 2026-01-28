use std::marker::PhantomData;

use crate::analysis::pcode_store::PcodeOpRef;

use crate::analysis::cpa::{ConfigurableProgramAnalysis, state::AbstractState};

/// Trait for collecting global analysis results (a.k.a. residues).
///
/// `Residue` provides hooks that the CPA algorithm calls while exploring the
/// program's abstract state space. Implementors can accumulate global program
/// information that isn't naturally stored in the abstract states themselves
/// and return that accumulated information in a structured way.
///
/// The hooks receive an `Option<PcodeOpRef<'_>>` describing the p-code operation
/// associated with the transition (if any).
///
/// Example: a reducer that records transitions into a CFG can use the op ref
/// to add edge payloads without forcing stores to always clone operations:
///
/// Notes:
/// - `new_state` is called for every observed transition A => B before merging.
/// - `merged_state` is called when the CPA merges two states; it receives the
///   current state, the original destination state, the merged state, and the
///   p-code operation (if present) that caused the transition.
///
/// ```
pub trait Residue<S> {
    type Output;

    /// Called for every observed transition (A => B) prior to merging.
    /// `op` is the optional pcode operation associated with the transition.
    fn new_state(&mut self, _state: &S, _dest_state: &S, _op: &Option<PcodeOpRef<'_>>) {}

    /// Called when two abstract states are merged. `curr_state` is the state
    /// that produced the transition, `original_merged_state` is the pre-merge
    /// destination, and `merged_state` is the state after merging. `op` is the
    /// optional p-code operation for the transition.
    fn merged_state(
        &mut self,
        _curr_state: &S,
        _original_merged_state: &S,
        _merged_state: &S,
        _op: &Option<PcodeOpRef<'_>>,
    ) {
    }

    /// Construct a new instance of the residue collector.
    fn new() -> Self;

    /// Finalize and return the collected output.
    fn finalize(self) -> Self::Output;
}
pub struct EmptyResidue<T>(PhantomData<T>);

impl<T: AbstractState> Residue<T> for EmptyResidue<T> {
    type Output = ();
    fn new() -> Self {
        Self(Default::default())
    }

    fn finalize(self) -> Self::Output {}
}

pub struct ResidueWrapper<A: ConfigurableProgramAnalysis, R: Residue<A::State>> {
    a: A,
    _phantom: PhantomData<R>,
}

impl<A: ConfigurableProgramAnalysis, R: Residue<A::State>> ResidueWrapper<A, R> {
    pub fn wrap(a: A, _r: R) -> Self {
        Self {
            a,
            _phantom: Default::default(),
        }
    }
}

impl<A: ConfigurableProgramAnalysis, R: Residue<A::State>> ConfigurableProgramAnalysis
    for ResidueWrapper<A, R>
{
    type State = A::State;

    type Reducer = R;
}

/// Delegate `IntoState` for the wrapper so callers can pass the same initial
/// input they would for the inner analysis when invoking `run` on the wrapper.
///
/// This implementation forwards construction of the initial state to the inner
/// analysis instance stored in the `ResidueWrapper`.
impl<T, A, R> crate::analysis::cpa::IntoState<ResidueWrapper<A, R>> for T
where
    A: crate::analysis::cpa::ConfigurableProgramAnalysis,
    R: Residue<A::State>,
    T: crate::analysis::cpa::IntoState<A> + Clone,
{
    fn into_state(
        self,
        c: &ResidueWrapper<A, R>,
    ) -> <ResidueWrapper<A, R> as crate::analysis::cpa::ConfigurableProgramAnalysis>::State {
        // Delegate to the inner analysis `A`
        self.into_state(&c.a)
    }
}
