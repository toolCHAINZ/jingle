use super::Residue;
use crate::analysis::cpa::state::AbstractState;

/// A simple reducer that records every visited destination state in a `Vec`.
///
/// This reducer does nothing as the CPA is running and at the end returns
/// a Vector of visited states given by the CPA
#[derive(Default)]
pub struct VecReducer;

impl<'a, S> Residue<'a, S> for VecReducer
where
    S: AbstractState,
{
    type Output = Vec<S>;

    /// Record the destination state index.
    ///
    /// The reducer stores the index of the destination state in the order
    /// transitions are observed by the CPA.
    fn new_state(
        &mut self,
        _source_idx: usize,
        _dest_idx: usize,
        _op: &Option<crate::analysis::pcode_store::PcodeOpRef<'a>>,
    ) {
    }

    /// When states are merged, we don't need to update our indices since
    /// the reached vector is updated in place by the CPA algorithm.
    fn merged_state(
        &mut self,
        _source_idx: usize,
        _merged_idx: usize,
        _op: &Option<crate::analysis::pcode_store::PcodeOpRef<'a>>,
    ) {
    }

    fn new() -> Self {
        Self
    }

    /// Return the collected visited states by indexing into the reached vector.
    fn finalize(self, reached: Vec<S>) -> Self::Output {
        reached
    }
}

/// Zero-sized factory for constructing `VecReducer` instances.
///
/// Exported as a public value `VEC` so callers can write:
/// ```ignore
/// let wrapped = analysis.with_residue(VEC);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct VecReducerFactory;

impl VecReducerFactory {
    /// Const-friendly constructor for the factory.
    pub const fn new() -> Self {
        VecReducerFactory
    }
}

impl Default for VecReducerFactory {
    fn default() -> Self {
        VecReducerFactory::new()
    }
}

/// Ergonomic public constant factory value.
pub const VEC: VecReducerFactory = VecReducerFactory;

/// Implement the reducer factory trait to allow the CPA wrapper machinery to
/// instantiate a `VecReducer<'op, A::State>` for any analysis `A`.
impl<A> super::ReducerFactoryForState<A> for VecReducerFactory
where
    A: crate::analysis::cpa::ConfigurableProgramAnalysis,
    A::State: AbstractState,
{
    type Reducer<'op> = VecReducer;

    fn make<'op>(&self) -> Self::Reducer<'op> {
        VecReducer
    }
}
