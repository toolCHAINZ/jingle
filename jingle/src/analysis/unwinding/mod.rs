mod cfg;

use crate::analysis::Analysis;
use crate::analysis::back_edge::{BackEdge, BackEdgeCPA, BackEdges};
use crate::analysis::cfg::{CfgState, ModeledPcodeCfg, PcodeCfg};
use crate::analysis::compound::{Strengthen, StrengthenOutcome};
use crate::analysis::cpa::lattice::flat::FlatLattice;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::lattice::simple::SimpleLattice;
use crate::analysis::cpa::lattice::{JoinSemiLattice, PartialJoinSemiLattice};
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, RunnableConfigurableProgramAnalysis};
use crate::analysis::pcode_store::PcodeStore;
use crate::analysis::unwinding::UnwoundLocation::{Location, UnwindError};
use crate::modeling::machine::MachineState;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use petgraph::visit::EdgeRef;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Formatter, LowerHex};
use std::iter::{empty, once};

impl PartialEq for BackEdgeVisitCountState {
    fn eq(&self, other: &Self) -> bool {
        self.same_visit_counts(other)
    }
}

impl PartialOrd for BackEdgeVisitCountState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.same_visit_counts(other) {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

impl PartialJoinSemiLattice for BackEdgeVisitCountState {
    fn partial_join(&self, other: &Self) -> Option<Self> {
        let mut visits = HashMap::new();
        for (addr, count) in self.back_edge_visits.iter() {
            let count = *count;
            let max: usize = count.max(other.back_edge_visits.get(addr).cloned().unwrap_or(0));
            visits.insert(*addr, max);
        }
        let s = Self {
            back_edge_visits: visits,
            max: self.max,
        };
        Some(s)
    }
}

impl JoinSemiLattice for BackEdgeVisitCountState {
    fn join(&mut self, other: &Self) {
        for (addr, count) in self.back_edge_visits.iter_mut() {
            let max: usize = other.back_edge_visits.get(addr).cloned().unwrap_or(0);
            *count = max;
        }
    }
}

impl<L: LocationState> Strengthen<L> for BackEdgeVisitCountState {
    fn strengthen(
        &mut self,
        original: &(Self, L),
        other: &L,
        _op: &PcodeOperation,
    ) -> StrengthenOutcome {
        let original_l = &original.1;
        let new_l = other;
        // if the edge is in the back edge
        if let Some(edge) = original_l.get_location().zip(new_l.get_location()) {
            if self.back_edge_visits.contains_key(&edge) {
                self.increment_back_edge_count(edge);
                StrengthenOutcome::Changed
            } else {
                StrengthenOutcome::Unchanged
            }
        } else {
            StrengthenOutcome::Unchanged
        }
    }
}

impl AbstractState for BackEdgeVisitCountState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }
    /// The actual work will be done in the sharpening operator
    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, _: B) -> Successor<'a, Self> {
        once(self.clone()).into()
    }
}

pub struct UnwoundLocationCPA {
    pub unwound_cfg: PcodeCfg<UnwoundLocation, PcodeOperation>,
    unwinding_bound: usize,
    max_step_bound: Option<usize>,
}

impl UnwoundLocationCPA {
    pub fn new(
        info: SleighArchInfo,
        unwinding_bound: usize,
        max_step_bound: Option<usize>,
    ) -> Self {
        Self {
            unwound_cfg: PcodeCfg::new(info),
            unwinding_bound,
            max_step_bound,
        }
    }

    /// Take ownership of the built unwound CFG, replacing it with an empty one
    pub fn take_cfg(&mut self) -> PcodeCfg<UnwoundLocation, PcodeOperation> {
        let info = self.unwound_cfg.info.clone();
        std::mem::replace(&mut self.unwound_cfg, PcodeCfg::new(info))
    }

    /// Inherent constructor for the analysis initial state.
    ///
    /// The old `Analysis` trait previously provided an associated `Input` and
    /// `make_initial_state` method. That interface was removed, so provide an
    /// inherent helper to construct the appropriate initial `State` for this
    /// analysis using the analysis instance (so it can capture bounds).
    pub fn make_initial_state(
        &self,
        addr: ConcretePcodeAddress,
    ) -> <Self as ConfigurableProgramAnalysis>::State {
        SimpleLattice::Value(UnwindingCpaState::new(
            addr,
            BackEdges::default(),
            self.unwinding_bound,
            self.max_step_bound,
        ))
    }
}

impl ConfigurableProgramAnalysis for UnwoundLocationCPA {
    type State = SimpleLattice<UnwindingCpaState>;

    fn reduce(
        &mut self,
        state: &Self::State,
        dest_state: &Self::State,
        op: &Option<PcodeOperation>,
    ) {
        if let SimpleLattice::Value(a) = state {
            let a = UnwoundLocation::from_cpa_state(a, a.max());
            self.unwound_cfg.add_node(&a);
            if !a.is_unwind_error() {
                if let Some(op) = op {
                    let dest = UnwoundLocation::from_cpa_state(
                        dest_state.value().unwrap(),
                        dest_state.value().unwrap().max(),
                    );
                    self.unwound_cfg.add_edge(a, dest, op)
                }
            }
        }
    }

    fn merged(
        &mut self,
        state: &Self::State,
        dest_state: &Self::State,
        merged_state: &Self::State,
        op: &Option<PcodeOperation>,
    ) {
        // Convert lattice states to UnwoundLocation
        let src =
            UnwoundLocation::from_cpa_state(state.value().unwrap(), state.value().unwrap().max());
        let dst = UnwoundLocation::from_cpa_state(
            dest_state.value().unwrap(),
            dest_state.value().unwrap().max(),
        );
        let merged = UnwoundLocation::from_cpa_state(
            merged_state.value().unwrap(),
            merged_state.value().unwrap().max(),
        );
        let op = op.clone().unwrap();
        // Find node indices
        let src_idx = match self.unwound_cfg.indices.get(&src) {
            Some(idx) => *idx,
            None => return,
        };
        let dst_idx = match self.unwound_cfg.indices.get(&dst) {
            Some(idx) => *idx,
            None => return,
        };
        // Find all edges from src to dst
        let mut edges_to_remove = Vec::new();
        for edge in self.unwound_cfg.graph.edges(src_idx) {
            if edge.target() == dst_idx {
                edges_to_remove.push(edge.id());
                // Get the operation for src (if any)
            }
        }
        // Remove edges from src to dst
        for edge_id in edges_to_remove {
            self.unwound_cfg.graph.remove_edge(edge_id);
        }
        // Add edges from src to merged with the same operation(s)
        self.unwound_cfg.add_edge(src, merged, op);
    }
}

impl Analysis for UnwoundLocationCPA {}

// Helper method for custom run logic
impl UnwoundLocationCPA {
    /// Run the unwinding CPA, first computing back-edges and then using those
    /// to build the unwound CFG. The `initial_state` can be any type that
    /// converts into the CPA `State` (for example, a `ConcretePcodeAddress` or
    /// a `SimpleLattice<UnwindingCpaState>`).
    pub fn run_with_back_edges<
        T: PcodeStore,
        I: Into<<Self as ConfigurableProgramAnalysis>::State>,
    >(
        &mut self,
        store: T,
        initial_state: I,
    ) -> Vec<<Self as ConfigurableProgramAnalysis>::State> {
        // Get the address from the initial state
        let init_lattice: <Self as ConfigurableProgramAnalysis>::State = initial_state.into();
        let addr = if let SimpleLattice::Value(ref state) = init_lattice {
            state.location()
        } else {
            panic!("Initial state must be a value")
        };

        // First run back edge analysis
        let mut back_edge_cpa = BackEdgeCPA::new();
        use crate::analysis::RunnableAnalysis as _;
        back_edge_cpa.run(&store, addr);
        let back_edges = back_edge_cpa.get_back_edges();

        // Create proper initial state with back edges
        let init_state =
            UnwindingCpaState::new(addr, back_edges, self.unwinding_bound, self.max_step_bound);

        let states = self.run_cpa(SimpleLattice::Value(init_state), &store);

        let graph = &mut self.unwound_cfg.graph;
        // For each node, process outgoing edges
        for node_idx in graph.node_indices() {
            // Map: location -> (count, edge_id)
            let mut location_to_edges: HashMap<_, Vec<petgraph::graph::EdgeIndex>> = HashMap::new();
            for edge in graph.edges(node_idx).collect::<Vec<_>>() {
                let target_idx = edge.target();
                if let Some(target_node) = graph.node_weight(target_idx) {
                    let loc = *target_node.location();
                    location_to_edges.entry(loc).or_default().push(edge.id());
                }
            }
        }
        self.make_output(states)
    }
}

pub type UnwindingAnalysis = UnwoundLocationCPA;

impl UnwindingAnalysis {
    pub fn new_with_bounds<T: PcodeStore>(pcode: &T, max: usize) -> Self {
        Self::new(pcode.info(), max, None)
    }

    pub fn with_step_bound(mut self, max_steps: usize) -> Self {
        self.max_step_bound = Some(max_steps);
        self
    }
}
