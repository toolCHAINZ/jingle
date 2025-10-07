use crate::JingleError;
use crate::analysis::Analysis;
use crate::analysis::cfg::{CfgState, CfgStateModel, ModelTransition, PcodeCfg};
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
use z3::ast::Bool;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum UnwoundLocation {
    UnwindError(ConcretePcodeAddress),
    Location(usize, ConcretePcodeAddress),
}

impl UnwoundLocation {
    pub fn count(&self) -> Option<usize> {
        match self {
            UnwindError(_) => None,
            Location(c, _) => Some(*c),
        }
    }
}

impl LowerHex for UnwoundLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:x}-{}",
            self.location(),
            self.count()
                .map(|a| format!("{:x}", a))
                .unwrap_or("STOP".to_string())
        )
    }
}

#[derive(Debug, Clone, Eq)]
pub struct UnwindingCpaState {
    location: ConcretePcodeAddress,
    visits: HashMap<ConcretePcodeAddress, usize>,
    max: usize,
}

impl UnwindingCpaState {
    pub fn new(location: ConcretePcodeAddress, max: usize) -> Self {
        let mut s = UnwindingCpaState {
            location,
            visits: Default::default(),
            max,
        };
        s.increment_visit_count();
        s
    }

    pub fn location(&self) -> ConcretePcodeAddress {
        self.location
    }

    pub fn visit_count(&self) -> usize {
        *self.visits.get(&self.location).unwrap_or(&0)
    }

    pub fn increment_visit_count(&mut self) {
        let new = self.visit_count() + 1;
        self.visits.insert(self.location, new);
    }
}

impl PartialEq for UnwindingCpaState {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location
    }
}
impl PartialOrd for UnwindingCpaState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.location() == other.location() {
            if self
                .visits
                .iter()
                .all(|v| other.visits.get(v.0).is_some_and(|c| c == v.1))
                && other
                    .visits
                    .iter()
                    .all(|v| self.visits.get(v.0).is_some_and(|c| c == v.1))
            {
                Some(Ordering::Equal)
            } else if self
                .visits
                .iter()
                .all(|v| other.visits.get(v.0).is_some_and(|c| c >= v.1))
            {
                Some(Ordering::Less)
            } else if other
                .visits
                .iter()
                .all(|v| self.visits.get(v.0).is_some_and(|c| c >= v.1))
            {
                Some(Ordering::Greater)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl PartialJoinSemiLattice for UnwindingCpaState {
    fn partial_join(&self, other: &Self) -> Option<Self> {
        if self.location == other.location {
            let mut visits = HashMap::new();
            for (addr, count) in self.visits.iter() {
                let count = *count;
                let max: usize = count.max(other.visits.get(addr).cloned().unwrap_or(0));
                visits.insert(*addr, max);
            }
            let s = Self {
                location: self.location,
                visits,
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
            for (addr, count) in self.visits.iter_mut() {
                let max: usize = other.visits.get(addr).cloned().unwrap_or(0);
                *count = max;
            }
        }
    }
}

impl AbstractState for UnwindingCpaState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        if self.location == other.location {
            let mut merged = MergeOutcome::NoOp;
            for (addr, count) in &other.visits {
                let self_count = self.visits.get(addr).cloned().unwrap_or(0);
                if count > &self_count {
                    self.visits.insert(*addr, *count);
                    merged = MergeOutcome::Merged;
                }
            }
            merged
        } else {
            MergeOutcome::NoOp
        }
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }
    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, opcode: B) -> Successor<'a, Self> {
        if self.location.machine == 0x1000005f0 {
            println!("There! {:x}:{:x}", self.location, self.visit_count());
        }
        if self.visit_count() > self.max {
            return std::iter::empty().into();
        }
        self.location
            .transfer(opcode.borrow())
            .into_iter()
            .map(|location| {
                let visits = self.visits.clone();
                let mut next = Self {
                    location,
                    visits,
                    max: self.max,
                };

                next.increment_visit_count();
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

    pub fn new(loc: &ConcretePcodeAddress, count: &usize) -> Self {
        Self::Location(*count, *loc)
    }

    pub fn is_unwind_error(&self) -> bool {
        matches!(self, UnwindError(_))
    }

    pub fn from_cpa_state(a: &UnwindingCpaState, max: usize) -> Self {
        if a.visit_count() > max {
            UnwindError(a.location())
        } else {
            Location(a.visit_count(), a.location())
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
        let pc = self.state.pc().eq(other.state.pc());
        pc
    }

    fn mem_eq(&self, other: &Self) -> Bool {
        let unwind = self.is_unwind_error.eq(&other.is_unwind_error);
        unwind & self.state.mem_eq(&other.state)
    }

    fn apply(&self, op: &PcodeOperation) -> Result<Self, JingleError> {
        Ok(UnwoundLocationModel {
            is_unwind_error: Bool::fresh_const("u"),
            state: self.state.apply(op)?,
        })
    }
}
impl CfgState for UnwoundLocation {
    type Model = UnwoundLocationModel;

    fn fresh(&self, i: &SleighArchInfo) -> Self::Model {
        let state = MachineState::fresh(i);
        UnwoundLocationModel {
            state,
            is_unwind_error: Bool::from_bool(self.is_unwind_error()),
        }
    }
}

pub type UnwoundCfg = PcodeCfg<UnwoundLocation, PcodeOperation>;

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
            self.unwound_cfg.add_node(a);
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
        println!("Remap edge from {:x} to {:x} to {:x}", src, dst, merged);
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
        let mut cpa = UnwoundLocationCPA {
            source_cfg: store,
            unwound_cfg: Default::default(),
        };
        let init_state = UnwindingCpaState::new(addr, self.max);
        let _ = cpa.run_cpa(&SimpleLattice::Value(init_state));

        let graph = &mut cpa.unwound_cfg.graph;
        // For each node, process outgoing edges
        for node_idx in graph.node_indices() {
            // Map: location -> (count, edge_id)
            let mut location_to_edges: HashMap<_, Vec<(usize, petgraph::graph::EdgeIndex)>> =
                HashMap::new();
            for edge in graph.edges(node_idx).collect::<Vec<_>>() {
                let target_idx = edge.target();
                if let Some(target_node) = graph.node_weight(target_idx) {
                    let loc = target_node.location().clone();
                    let count = target_node.count().unwrap_or(0);
                    location_to_edges
                        .entry(loc)
                        .or_default()
                        .push((count, edge.id()));
                }
            }
            // For each location, keep only the edge with the highest count
            for (_loc, mut edges) in location_to_edges {
                if edges.len() <= 1 {
                    continue;
                }
                // Sort by count descending
                edges.sort_by(|a, b| b.0.cmp(&a.0));
                // Keep the first (highest count), remove the rest
                for &(_count, edge_id) in edges.iter().skip(1) {
                    graph.remove_edge(edge_id);
                }
            }
        }
        cpa.unwound_cfg
    }

    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input {
        addr
    }
}
