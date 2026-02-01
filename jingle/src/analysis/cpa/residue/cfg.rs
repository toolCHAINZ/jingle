use crate::analysis::cfg::CfgState;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::LocationState;
use crate::analysis::pcode_store::PcodeOpRef;

/// A generic reducer that records transitions/merges into a `PcodeCfg`.
///
/// This reducer observes CPA transitions and builds a `PcodeCfg` whose nodes
/// are the abstract states (`N`) and whose edge payloads are `PcodeOpRef<'a>`.
#[derive(Debug)]
pub struct CfgReducer<'a, N>
where
    N: LocationState + CfgState,
{
    /// The accumulated CFG.
    pub cfg: PcodeCfg<N, PcodeOpRef<'a>>,
}

impl<'a, N> Default for CfgReducer<'a, N>
where
    N: LocationState + CfgState,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, N> CfgReducer<'a, N>
where
    N: LocationState + CfgState,
{
    /// Create an empty `CfgReducer`.
    pub fn new() -> Self {
        Self {
            cfg: PcodeCfg::new(),
        }
    }

    /// Take ownership of the built CFG, leaving an empty one in its place.
    pub fn take_cfg(&mut self) -> PcodeCfg<N, PcodeOpRef<'a>> {
        std::mem::take(&mut self.cfg)
    }
}

impl<'a, N> Residue<'a, N> for CfgReducer<'a, N>
where
    N: LocationState + CfgState,
{
    type Output = PcodeCfg<N, PcodeOpRef<'a>>;

    /// Record a transition from `state` to `dest_state`, optionally with `op`.
    fn new_state(&mut self, state: &N, dest_state: &N, op: &Option<PcodeOpRef<'a>>) {
        // Ensure the source node exists
        self.cfg.add_node(state);

        if let Some(op_ref) = op {
            // Clone the referenced op into an owned payload for the edge
            let owned = op_ref.clone();
            self.cfg.add_edge(state, dest_state, owned);
        }
    }

    /// Handle merges by updating nodes/edges in the internal CFG.
    fn merged_state(
        &mut self,
        state: &N,
        dest_state: &N,
        merged_state: &N,
        op: &Option<PcodeOpRef<'a>>,
    ) {
        // Replace occurrences of `dest_state` with `merged_state` in the CFG
        self.cfg.replace_and_combine_nodes(dest_state, merged_state);

        if let Some(op_ref) = op {
            let owned = op_ref.clone();
            self.cfg.add_edge(state, merged_state, owned);
        }
    }

    fn new() -> Self {
        Self::new()
    }

    fn finalize(self) -> Self::Output {
        self.cfg
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
    type Reducer<'op> = CfgReducer<'op, A::State>;

    fn make<'op>(&self) -> Self::Reducer<'op> {
        CfgReducer {
            cfg: PcodeCfg::new(),
        }
    }
}
