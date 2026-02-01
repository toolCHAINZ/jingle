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
/// Notes:
/// - `new_state` is called for every observed transition A => B before merging.
/// - `merged_state` is called when the CPA merges two states; it receives the
///   current state, the original destination state, the merged state, and the
///   p-code operation (if present) that caused the transition.
pub trait Residue<'a, S> {
    type Output;

    /// Called for every observed transition (A => B) prior to merging.
    /// `op` is the optional pcode operation associated with the transition.
    fn new_state(&mut self, _state: &S, _dest_state: &S, _op: &Option<PcodeOpRef<'a>>) {}

    /// Called when two abstract states are merged. `curr_state` is the state
    /// that produced the transition, `original_merged_state` is the pre-merge
    /// destination, and `merged_state` is the state after merging. `op` is the
    /// optional p-code operation for the transition.
    fn merged_state(
        &mut self,
        _curr_state: &S,
        _original_merged_state: &S,
        _merged_state: &S,
        _op: &Option<PcodeOpRef<'a>>,
    ) {
    }

    /// Construct a new instance of the residue collector.
    fn new() -> Self;

    /// Finalize and return the collected output.
    fn finalize(self) -> Self::Output;
}

pub struct EmptyResidue<T>(PhantomData<T>);

impl<'a, T: AbstractState> Residue<'a, T> for EmptyResidue<T> {
    type Output = ();
    fn new() -> Self {
        Self(Default::default())
    }

    fn finalize(self) -> Self::Output {}
}

/// A factory trait: given an analysis `A` we need a way to produce a reducer
/// instance specialized for any p-code-op borrow lifetime `'op`.
///
/// Implementations of this trait should be ZSTs (zero-sized types) that can
/// construct a reducer instance for a specific `'op` when requested.
///
/// Example:
/// ```ignore
/// pub struct CfgReducerFactory;
/// impl<A> ReducerFactoryForState<A> for CfgReducerFactory where A: ConfigurableProgramAnalysis {
///     type Reducer<'op> = crate::analysis::cpa::reducer::CfgReducer<'op, A::State>;
///     fn make<'op>(&self) -> Self::Reducer<'op> { crate::analysis::cpa::reducer::CfgReducer::new() }
/// }
/// ```
pub trait ReducerFactoryForState<A: ConfigurableProgramAnalysis> {
    /// The reducer type for a given `'op` lifetime.
    type Reducer<'op>: Residue<'op, A::State>;

    /// Create a reducer instance for lifetime `'op`.
    fn make<'op>(&self) -> Self::Reducer<'op>;
}

/// ResidueWrapper now stores a reducer *factory* rather than a concrete reducer
/// type. This allows the wrapper to instantiate reducers specialized to the
/// pcode-store borrow lifetime `'op` when `run_cpa` is invoked.
///
/// The factory `F` must produce a reducer type `F::Reducer<'op>` that implements
/// `Residue<'op, A::State>` for any `'op` (expressed via the GAT on the factory).
pub struct ResidueWrapper<A: ConfigurableProgramAnalysis, F>
where
    for<'op> F: ReducerFactoryForState<A>,
{
    a: A,
    factory: F,
    _phantom: PhantomData<F>,
}

impl<A: ConfigurableProgramAnalysis, F> ResidueWrapper<A, F>
where
    for<'op> F: ReducerFactoryForState<A>,
{
    /// Wrap an analysis together with a reducer factory.
    pub fn wrap(a: A, factory: F) -> Self {
        Self {
            a,
            factory,
            _phantom: Default::default(),
        }
    }

    /// Convenience: allow converting an existing factory to another factory
    /// via the `with_residue`-style chain. This mirrors the previous ergonomics.
    pub fn with_residue<G>(self, factory: G) -> ResidueWrapper<A, G>
    where
        for<'op> G: ReducerFactoryForState<A>,
    {
        ResidueWrapper {
            a: self.a,
            factory,
            _phantom: Default::default(),
        }
    }
}

impl<A: ConfigurableProgramAnalysis, F> ConfigurableProgramAnalysis for ResidueWrapper<A, F>
where
    for<'op> F: ReducerFactoryForState<A>,
{
    type State = A::State;

    // Expose the reducer GAT from the factory so callers of `run_cpa<'op>` can
    // instantiate `Reducer<'op>` from the factory for the requested lifetime.
    type Reducer<'op> = <F as ReducerFactoryForState<A>>::Reducer<'op>;
}

/// Delegate `IntoState` for the wrapper so callers can pass the same initial
/// input they would for the inner analysis when invoking `run` on the wrapper.
impl<T, A, F> crate::analysis::cpa::IntoState<ResidueWrapper<A, F>> for T
where
    A: crate::analysis::cpa::ConfigurableProgramAnalysis,
    for<'op> F: ReducerFactoryForState<A>,
    T: crate::analysis::cpa::IntoState<A> + Clone,
{
    fn into_state(
        self,
        c: &ResidueWrapper<A, F>,
    ) -> <ResidueWrapper<A, F> as crate::analysis::cpa::ConfigurableProgramAnalysis>::State {
        // Delegate to the inner analysis `A`
        self.into_state(&c.a)
    }
}
