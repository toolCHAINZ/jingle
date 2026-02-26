use crate::analysis::cfg::CfgState;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::LocationState;
use crate::analysis::pcode_store::PcodeOpRef;

/// A generic reducer that records transitions/merges into a `PcodeCfg`.
///
/// This reducer observes CPA transitions by tracking index pairs and associated
/// operations, then builds a `PcodeCfg` in `finalize()` using the actual states.
#[derive(Debug)]
pub struct CfgReducer<'a>
{
    /// Accumulated edges as (source_idx, dest_idx, optional_operation) tuples.
    edges: Vec<(usize, usize, Option<PcodeOpRef<'a>>)>,
}

impl<'a> Default for CfgReducer<'a>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> CfgReducer<'a>

{
    /// Create an empty `CfgReducer`.
    pub fn new() -> Self {
        Self {
            edges: Vec::new(),
        }
    }
}

impl<'a, N> Residue<'a, N> for CfgReducer<'a>
where
    N: LocationState + CfgState,
{
    type Output = PcodeCfg<N, PcodeOpRef<'a>>;

    /// Record a transition from source to destination state with optional operation.
    fn new_state(&mut self, source_idx: usize, dest_idx: usize, op: &Option<PcodeOpRef<'a>>) {
        // Clone the operation if present and store the edge
        self.edges.push((source_idx, dest_idx, op.clone()));
    }

    /// Handle merges by recording the edge from source to merged state.
    fn merged_state(&mut self, source_idx: usize, merged_idx: usize, op: &Option<PcodeOpRef<'a>>) {
        // Record the edge to the merged state
        self.edges.push((source_idx, merged_idx, op.clone()));
    }

    fn new() -> Self {
        Self::new()
    }

    /// Build the PcodeCfg from accumulated edges and the reached states.
    fn finalize(self, reached: Vec<N>) -> Self::Output {
        let mut cfg = PcodeCfg::new();
        dbg!(self.edges.len(), reached.len());
        // Add all edges, which will automatically add nodes as needed
        for (source_idx, dest_idx, op) in self.edges {
            if let Some(op_ref) = op {
                cfg.add_edge(&reached[source_idx], &reached[dest_idx], op_ref);
            } else {
                // Add nodes even without an operation edge
                cfg.add_node(&reached[source_idx]);
                cfg.add_node(&reached[dest_idx]);
            }
        }

        cfg
    }
}

/// Zero-sized factory for constructing `CfgReducer` instances.
///
/// Exported as a public zero-sized type so callers can pass the factory value
/// (or the `CFG` const) to APIs like `with_residue`.
#[derive(Debug, Clone, Copy)]
pub struct CfgReducerFactory;

impl CfgReducerFactory {
    /// Create a new factory value (const-friendly).
    pub const fn new() -> Self {
        CfgReducerFactory
    }
}

impl Default for CfgReducerFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Ergonomic public constant that can be passed to `with_residue(...)`.
pub const CFG: CfgReducerFactory = CfgReducerFactory;

/// Implement the reducer factory trait so this factory can be used by the CPA
/// wrapping mechanisms to instantiate `CfgReducer<'op, A::State>`.
impl<A> crate::analysis::cpa::residue::ReducerFactoryForState<A> for CfgReducerFactory
where
    A: crate::analysis::cpa::ConfigurableProgramAnalysis,
    A::State: crate::analysis::cpa::state::LocationState + CfgState,
{
    type Reducer<'op> = CfgReducer<'op>;

    fn make<'op>(&self) -> Self::Reducer<'op> {
        CfgReducer::new()
    }
}
