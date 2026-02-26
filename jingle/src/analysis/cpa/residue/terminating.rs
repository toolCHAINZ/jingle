use std::collections::HashSet;
use std::marker::PhantomData;

use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::AbstractState;

/// A reducer that collects terminating states: states that never transition to other states.
///
/// A state is considered terminating if it has no successors when the transfer function
/// is applied.
///
/// The reducer tracks state indices: all destination indices and which indices appear
/// as sources (non-terminating). In `finalize()`, the difference gives terminating states.
pub struct TerminatingReducer<S>
where
    S: AbstractState,
{
    /// Indices of states that have been observed as source states (i.e., have successors).
    terminating_indices: HashSet<usize>,
    _phantom: PhantomData<S>,
}

impl<S> Default for TerminatingReducer<S>
where
    S: AbstractState,
{
    fn default() -> Self {
        Self {
            terminating_indices: HashSet::new(),
            _phantom: Default::default(),
        }
    }
}

impl<'a, S> Residue<'a, S> for TerminatingReducer<S>
where
    S: AbstractState,
{
    type Output = Vec<S>;

    /// Track a state transition from source to destination.
    ///
    /// The source state has at least one successor, so it is not terminating.
    /// The destination state is a potential terminating state (to be verified later).
    fn new_state(
        &mut self,
        source_idx: usize,
        dest_idx: usize,
        _op: &Option<crate::analysis::pcode_store::PcodeOpRef<'a>>,
    ) {
        // The source state has successors, so it's not terminating
        self.terminating_indices.remove(&source_idx);
        // For now, consider dest as terminating
        self.terminating_indices.insert(dest_idx);
    }

    /// Handle state merging by tracking the source as non-terminating.
    ///
    /// The source state produced a transition, so it's not terminating.
    fn merged_state(
        &mut self,
        source_idx: usize,
        _merged_idx: usize,
        _op: &Option<crate::analysis::pcode_store::PcodeOpRef<'a>>,
    ) {
        // The source state has successors, so it's not terminating
        self.terminating_indices.remove(&source_idx);
    }

    fn new() -> Self {
        Self::default()
    }

    /// Return the collected terminating states.
    ///
    /// Terminating states are those indices that appear in all_indices but not
    /// in non_terminating_indices.
    fn finalize(self, reached: Vec<S>) -> Self::Output {
        self.terminating_indices
            .into_iter()
            .map(|idx| reached[idx].clone())
            .collect()
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
