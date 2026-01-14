use crate::analysis::cfg::CfgState;
use crate::analysis::cfg::PcodeCfg;
use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::LocationState;
use jingle_sleigh::PcodeOperation;
use petgraph::visit::EdgeRef;

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
pub struct CfgReducer<N>
where
    N: LocationState + CfgState,
{
    /// The constructed Pcode CFG capturing reductions observed during analysis.
    pub cfg: PcodeCfg<N, PcodeOperation>,
}

impl<N> CfgReducer<N>
where
    N: LocationState + CfgState,
{
    /// Take ownership of the built CFG, replacing it with an empty one.
    pub fn take_cfg(&mut self) -> PcodeCfg<N, PcodeOperation> {
        std::mem::take(&mut self.cfg)
    }
}

impl<N> Residue<N> for CfgReducer<N>
where
    N: LocationState + CfgState,
{
    type Output = PcodeCfg<N>;
    /// Record a reduction/transition from `state` to `dest_state` into the CFG.
    ///
    /// This mirrors the logic previously implemented in the unwinding CPA's
    /// `reduce` method, but generalized: we convert both CPA states into `N`
    /// via the mapper and add nodes/edges to the cfg. If `op` is `None`,
    /// only the source node is added (no edge).
    fn residue(&mut self, state: &N, dest_state: &N, op: &Option<PcodeOperation>) {
        self.cfg.add_node(state);

        if let Some(op) = op {
            // add_edge will insert nodes if missing
            self.cfg.add_edge(state, dest_state, op.clone());
        }
    }

    /// When two abstract states are merged in the CPA, adjust the recorded CFG
    /// so edges that previously pointed to the original `dest_state` now point
    /// to the `merged_state`.
    ///
    /// This duplicates the behavior from the unwinding CPA's `merged` method in
    /// a generic way.
    fn merged(
        &mut self,
        _state: &N,
        dest_state: &N,
        merged_state: &N,
        _op: &Option<PcodeOperation>,
    ) {
        // If operation is not present we can't deterministically reconstruct
        // a replacement edge payload; however we still should remove edges
        // from src->dst and add a new edge with no-op payload is not supported.
        // We only proceed when there is an op provided (matches unwinding impl).
        self.cfg.replace_node(dest_state, merged_state);
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
