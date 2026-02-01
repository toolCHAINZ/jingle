use crate::analysis::cfg::CfgState;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::LocationState;
use crate::analysis::pcode_store::PcodeOpRef;

/// A generic reducer that adapts an arbitrary CPA into a "reducer" which
/// records reductions/transitions into a `PcodeCfg`.
///
/// The reducer wraps another CPA `A` and uses a user-supplied mapping function
/// to convert abstract states (`A::State`) into CFG nodes (`N`) stored in the
/// `PcodeCfg`. Whenever the CPA observes a transition (via `residue`) or a
/// merge (via `merged`), the reducer records corresponding nodes/edges in the
/// contained CFG.
///
/// Notes:
/// - This type fixes the edge payload type to `PcodeOperation`, matching the
///   common use-case of recording p-code transitions.
/// - The reducer does not attempt to modify the wrapped CPA's behavior; it
///   only observes the CPA via the `residue` and `merged` hooks and updates the
///   CFG accordingly.
pub struct CfgReducer<'a, N>
where
    N: LocationState + CfgState,
{
    /// The constructed Pcode CFG capturing reductions observed during analysis.
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
    /// Create an empty `CfgReducer` for this lifetime and state type.
    pub fn new() -> Self {
        Self {
            cfg: PcodeCfg::new(),
        }
    }

    /// Take ownership of the built CFG, replacing it with an empty one.
    pub fn take_cfg(&mut self) -> PcodeCfg<N, PcodeOpRef<'a>> {
        std::mem::take(&mut self.cfg)
    }
}

impl<'a, N> Residue<'a, N> for CfgReducer<'a, N>
where
    N: LocationState + CfgState,
{
    type Output = PcodeCfg<N, PcodeOpRef<'a>>;
    /// Record a reduction/transition from `state` to `dest_state` into the CFG.
    ///
    /// This mirrors the logic previously implemented in the unwinding CPA's
    /// `reduce` method, but generalized: we convert both CPA states into `N`
    /// via the mapper and add nodes/edges to the cfg. If `op` is `None`,
    /// only the source node is added (no edge).
    fn new_state(&mut self, state: &N, dest_state: &N, op: &Option<PcodeOpRef<'a>>) {
        self.cfg.add_node(state);

        if let Some(op) = op {
            // Convert the wrapped op into an owned PcodeOperation before inserting.
            // `PcodeOpRef` derefs to `PcodeOperation`; call `as_ref().clone()` to obtain an owned op.
            let owned_op = op.clone();
            // add_edge will insert nodes if missing
            self.cfg.add_edge(state, dest_state, owned_op);
        }
    }

    /// When two abstract states are merged in the CPA, adjust the recorded CFG
    /// so edges that previously pointed to the original `dest_state` now point
    /// to the `merged_state`.
    ///
    /// This duplicates the behavior from the unwinding CPA's `merged` method in
    /// a generic way.
    fn merged_state(
        &mut self,
        state: &N,
        dest_state: &N,
        merged_state: &N,
        op: &Option<PcodeOpRef<'a>>,
    ) {
        tracing::debug!("merged called: dest_state and merged_state provided");
        // If operation is not present we can't deterministically reconstruct
        // a replacement edge payload; however we still should remove edges
        // from src->dst and add a new edge with no-op payload is not supported.
        // We only proceed when there is an op provided (matches unwinding impl).
        self.cfg.replace_and_combine_nodes(dest_state, merged_state);
        if let Some(op) = op {
            let owned_op = op.clone();
            self.cfg.add_edge(state, merged_state, owned_op);
        }
    }

    fn new() -> Self {
        Self {
            cfg: PcodeCfg::new(),
        }
    }

    fn finalize(self) -> Self::Output {
        self.cfg
    }
}

/// Zero-sized factory for constructing `CfgReducer` instances specialized to a
/// particular p-code operation borrow lifetime `'op`. This ZST is intended to be
/// passed to `with_residue` so examples can write `with_residue(CfgReducerFactory)`.
pub struct CfgReducerFactory;

impl Default for CfgReducerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl CfgReducerFactory {
    pub const fn new() -> Self {
        CfgReducerFactory
    }
}

/// Public constant factory value for ergonomic usage.
///
/// Prefer passing `CFG` to `with_residue` in examples and user code:
/// ```ignore
/// let analysis_with_cfg = analysis.with_residue(CFG);
/// ```
pub const CFG: CfgReducerFactory = CfgReducerFactory;

impl<A> crate::analysis::cpa::residue::ReducerFactoryForState<A> for CfgReducerFactory
where
    A: crate::analysis::cpa::ConfigurableProgramAnalysis,
    A::State: crate::analysis::cpa::state::LocationState + crate::analysis::cfg::CfgState,
{
    type Reducer<'op> = crate::analysis::cpa::reducer::CfgReducer<'op, A::State>;

    fn make<'op>(&self) -> Self::Reducer<'op> {
        // Construct a reducer with an empty cfg. The concrete `'op` lifetime is
        // supplied by the caller (run_cpa) when instantiating the factory.
        crate::analysis::cpa::reducer::CfgReducer {
            cfg: PcodeCfg::new(),
        }
    }
}
