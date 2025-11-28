use crate::JingleError;
use crate::analysis::Analysis;
use crate::analysis::back_edge::{BackEdge, BackEdgeAnalysis, BackEdges};
use crate::analysis::cfg::{CfgState, CfgStateModel, PcodeCfg};
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::simple::SimpleLattice;
use crate::analysis::cpa::lattice::{JoinSemiLattice, PartialJoinSemiLattice};
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
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
use std::iter::empty;
use z3::ast::Bool;

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum UnwoundLocation {
    UnwindError(ConcretePcodeAddress),
    Location(String, ConcretePcodeAddress),
}

impl UnwoundLocation {}

impl LowerHex for UnwoundLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let tag = match self {
            UnwoundLocation::UnwindError(_) => "_Stop",
            UnwoundLocation::Location(a, _) => a.as_str(),
        };
        write!(f, "{:x}{}", self.location(), tag)
    }
}

#[derive(Debug, Clone, Eq)]
pub struct UnwindingCpaState {
    location: ConcretePcodeAddress,
    back_edge_visits: HashMap<(ConcretePcodeAddress, ConcretePcodeAddress), usize>,
    max: usize,
}

impl UnwindingCpaState {
    pub fn new(location: ConcretePcodeAddress, back_edges: BackEdges, max: usize) -> Self {
        UnwindingCpaState {
            location,
            back_edge_visits: back_edges.iter().map(|k| (k, 0)).collect(),
            max,
        }
    }

    pub fn back_edge_str(&self) -> String {
        let mut sorted = self
            .back_edge_visits
            .clone()
            .into_iter()
            .collect::<Vec<(BackEdge, usize)>>();
        sorted.sort_by(|(a, _), (b, _)| match a.0.cmp(&b.0) {
            Ordering::Equal => a.1.cmp(&b.1),
            a => a,
        });
        let strs: Vec<_> = sorted.iter().map(|(_, size)| size.to_string()).collect();
        strs.join("_")
    }
    pub fn location(&self) -> ConcretePcodeAddress {
        self.location
    }

    pub fn back_edge_count(&self, be: BackEdge) -> Option<usize> {
        self.back_edge_visits.get(&be).cloned()
    }
    pub fn increment_back_edge_count(&mut self, be: BackEdge) {
        if let Some(count) = self.back_edge_visits.get_mut(&be) {
            *count += 1;
        }
    }

    pub fn terminated(&self) -> bool {
        self.back_edge_visits.values().any(|b| b >= &self.max)
    }

    pub fn same_visit_counts(&self, other: &UnwindingCpaState) -> bool {
        self.back_edge_visits.eq(&other.back_edge_visits)
    }
}

impl PartialEq for UnwindingCpaState {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location
    }
}
impl PartialOrd for UnwindingCpaState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.location() == other.location() && self.same_visit_counts(other) {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

impl PartialJoinSemiLattice for UnwindingCpaState {
    fn partial_join(&self, other: &Self) -> Option<Self> {
        if self.location == other.location {
            let mut visits = HashMap::new();
            for (addr, count) in self.back_edge_visits.iter() {
                let count = *count;
                let max: usize = count.max(other.back_edge_visits.get(addr).cloned().unwrap_or(0));
                visits.insert(*addr, max);
            }
            let s = Self {
                location: self.location,
                back_edge_visits: visits,
                max: self.max,
            };
            Some(s)
        } else {
            None
        }
    }
}

impl JoinSemiLattice for UnwindingCpaState {
    fn join(&mut self, other: &Self) {
        if self.location == other.location {
            for (addr, count) in self.back_edge_visits.iter_mut() {
                let max: usize = other.back_edge_visits.get(addr).cloned().unwrap_or(0);
                *count = max;
            }
        }
    }
}

impl AbstractState for UnwindingCpaState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }
    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, opcode: B) -> Successor<'a, Self> {
        if self.terminated() {
            return empty().into();
        }
        self.location
            .transfer(opcode.borrow())
            .into_iter()
            .map(|location| {
                let mut next = self.clone();
                next.location = location;
                next.increment_back_edge_count((self.location, location));
                next
            })
            .into()
    }
}

impl LocationState for UnwindingCpaState {
    fn get_operation<T: PcodeStore>(&self, t: &T) -> Option<PcodeOperation> {
        t.get_pcode_op_at(self.location)
    }
}

impl UnwoundLocation {
    pub fn location(&self) -> &ConcretePcodeAddress {
        match self {
            UnwindError(a) => a,
            Location(_, a) => a,
        }
    }

    pub fn new<T: AsRef<str>>(loc: &ConcretePcodeAddress, count: T) -> Self {
        Self::Location(count.as_ref().into(), *loc)
    }

    pub fn is_unwind_error(&self) -> bool {
        matches!(self, UnwindError(_))
    }

    pub fn from_cpa_state(a: &UnwindingCpaState, _max: usize) -> Self {
        if a.terminated() {
            UnwindError(a.location())
        } else {
            Location(a.back_edge_str(), a.location())
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnwoundLocationModel {
    is_unwind_error: Bool,
    state: MachineState,
}

impl CfgStateModel for UnwoundLocationModel {
    fn location_eq(&self, other: &Self) -> Bool {
        self.state.pc().eq(other.state.pc())
    }

    fn state_eq(&self, other: &Self) -> Bool {
        let unwind = self.is_unwind_error.eq(&other.is_unwind_error);
        unwind & self.state.state_eq(&other.state)
    }

    fn apply(&self, op: &PcodeOperation) -> Result<Self, JingleError> {
        Ok(UnwoundLocationModel {
            is_unwind_error: Bool::fresh_const("u"),
            state: self.state.apply(op)?,
        })
    }
}
impl CfgState for UnwoundLocation {
    type Model = MachineState;

    fn fresh_model(&self, i: &SleighArchInfo) -> Self::Model {
        let state = MachineState::fresh(i);
       state
    }
    fn model_id(&self) -> String {
        format!("{:x}", self)
    }
}

pub type UnwoundPcodeCfg = PcodeCfg<UnwoundLocation, PcodeOperation>;

struct UnwoundLocationCPA<T: PcodeStore> {
    source_cfg: T,
    unwound_cfg: PcodeCfg<UnwoundLocation, PcodeOperation>,
}

impl<T: PcodeStore> ConfigurableProgramAnalysis for UnwoundLocationCPA<T> {
    type State = SimpleLattice<UnwindingCpaState>;

    fn get_pcode_store(&self) -> &impl PcodeStore {
        &self.source_cfg
    }

    fn reduce(&mut self, state: &Self::State, dest_state: &Self::State) {
        if let SimpleLattice::Value(a) = state {
            let a = UnwoundLocation::from_cpa_state(a, a.max);
            self.unwound_cfg.add_node(&a);
            if !a.is_unwind_error() {
                if let Some(op) = self.source_cfg.get_pcode_op_at(a.location()) {
                    let dest = UnwoundLocation::from_cpa_state(
                        dest_state.value().unwrap(),
                        dest_state.value().unwrap().max,
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
    ) {
        // Convert lattice states to UnwoundLocation
        let src =
            UnwoundLocation::from_cpa_state(state.value().unwrap(), state.value().unwrap().max);
        let dst = UnwoundLocation::from_cpa_state(
            dest_state.value().unwrap(),
            dest_state.value().unwrap().max,
        );
        let merged = UnwoundLocation::from_cpa_state(
            merged_state.value().unwrap(),
            merged_state.value().unwrap().max,
        );
        let op = self.source_cfg.get_pcode_op_at(src.location()).unwrap();
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

pub struct UnwindingAnalysis {
    max: usize,
}

impl UnwindingAnalysis {
    pub fn new(max: usize) -> Self {
        Self { max }
    }
}
impl Analysis for UnwindingAnalysis {
    type Output = PcodeCfg<UnwoundLocation, PcodeOperation>;
    type Input = ConcretePcodeAddress;

    fn run<T: PcodeStore, I: Into<Self::Input>>(
        &mut self,
        store: T,
        initial_state: I,
    ) -> Self::Output {
        let addr = initial_state.into();
        let bes = BackEdgeAnalysis.make_initial_state(addr);
        let back_edges = BackEdgeAnalysis.run(&store, bes);
        let mut cpa = UnwoundLocationCPA {
            source_cfg: store,
            unwound_cfg: Default::default(),
        };
        let init_state = UnwindingCpaState::new(addr, dbg!(back_edges), self.max);
        let _ = cpa.run_cpa(&SimpleLattice::Value(init_state));

        let graph = &mut cpa.unwound_cfg.graph;
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
        cpa.unwound_cfg
    }

    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input {
        addr
    }
}
