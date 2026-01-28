use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::hash::DefaultHasher;

pub type BackEdge = (ConcretePcodeAddress, ConcretePcodeAddress);

#[derive(Clone, Debug, Default)]
pub struct BackEdges {
    // todo: make generic?
    edges: std::sync::Arc<HashMap<ConcretePcodeAddress, HashSet<ConcretePcodeAddress>>>,
}

impl BackEdges {
    /// Ensure we have unique ownership of the inner map and return a mutable
    /// reference to it. Uses `Arc::make_mut` for copy-on-write semantics so
    /// clones share until mutated.
    fn ensure_unique(
        &mut self,
    ) -> &mut HashMap<ConcretePcodeAddress, HashSet<ConcretePcodeAddress>> {
        std::sync::Arc::make_mut(&mut self.edges)
    }

    pub fn has(&self, from: &ConcretePcodeAddress, to: &ConcretePcodeAddress) -> bool {
        self.edges.get(from).is_some_and(|s| s.contains(to))
    }

    pub fn add(&mut self, from: ConcretePcodeAddress, to: ConcretePcodeAddress) {
        let map = self.ensure_unique();
        map.entry(from).or_default().insert(to);
    }

    pub fn get_all_for(
        &self,
        from: &ConcretePcodeAddress,
    ) -> Option<HashSet<ConcretePcodeAddress>> {
        self.edges.get(from).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = BackEdge> + '_ {
        self.edges
            .iter()
            .flat_map(|(src, edges)| edges.iter().map(|dst| (*src, *dst)))
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct BackEdgeState {
    pub(crate) path_visits: HashSet<PcodeAddressLattice>,
    pub(crate) location: PcodeAddressLattice,
}

impl IntoState<BackEdgeCPA> for PcodeAddressLattice {
    fn into_state(self, _c: &BackEdgeCPA) -> <BackEdgeCPA as ConfigurableProgramAnalysis>::State {
        BackEdgeState {
            location: self,
            path_visits: Default::default(),
        }
    }
}

impl BackEdgeState {
    pub fn new(location: PcodeAddressLattice) -> BackEdgeState {
        Self {
            location,
            path_visits: Default::default(),
        }
    }
    pub fn add_location(&self, loc: PcodeAddressLattice) -> BackEdgeState {
        let mut s = self.clone();
        // Insert a clone into the visited set, then move the original into `s.location`.
        s.path_visits.insert(loc.clone());
        s.location = loc;
        s
    }
}

impl From<ConcretePcodeAddress> for BackEdgeState {
    fn from(addr: ConcretePcodeAddress) -> Self {
        Self::new(PcodeAddressLattice::Const(addr))
    }
}

impl PartialOrd for BackEdgeState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.location.value() == other.location.value() {
            let other_visits = other.path_visits.get(&other.location)?;
            let self_visits = self.path_visits.get(&self.location)?;
            if self_visits == other_visits {
                Some(Ordering::Equal)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl JoinSemiLattice for BackEdgeState {
    fn join(&mut self, _other: &Self) {
        // We don't use `join` on this state so no need to implement it
        unimplemented!()
    }
}

impl AbstractState for BackEdgeState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, opcode: B) -> Successor<'a, Self> {
        let opcode = opcode.borrow();
        self.location
            .transfer(opcode)
            .into_iter()
            .map(|a| self.add_location(a))
            .into()
    }
}

impl LocationState for BackEdgeState {
    fn get_operation<'a, T: crate::analysis::pcode_store::PcodeStore + ?Sized>(
        &'a self,
        t: &'a T,
    ) -> Option<crate::analysis::pcode_store::PcodeOpRef<'a>> {
        self.location.get_operation(t)
    }

    fn get_location(&self) -> Option<ConcretePcodeAddress> {
        self.location.value().cloned()
    }
}

impl Display for BackEdgeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hasher = DefaultHasher::new();
        self.path_visits.s
        write!(f, "{:x}{}", self.location, self.)
    }
}

/// A reducer that identifies back-edges during the analysis.
///
/// A back-edge is an edge from a state to a previously visited state in its path.
/// This reducer tracks all visited edges and identifies when a transition creates
/// a back-edge by checking if the destination location appears in the source state's
/// visited path.
pub struct BackEdgeReducer {
    /// All visited edges (from_location, to_location)
    visited_edges: Vec<(ConcretePcodeAddress, ConcretePcodeAddress)>,
    /// Identified back-edges
    back_edges: BackEdges,
}

impl BackEdgeReducer {
    pub fn new() -> Self {
        Self {
            visited_edges: Vec::new(),
            back_edges: BackEdges::default(),
        }
    }

    pub fn new_with_capacity(cap: usize) -> Self {
        Self {
            visited_edges: Vec::with_capacity(cap),
            back_edges: BackEdges::default(),
        }
    }
}

impl Default for BackEdgeReducer {
    fn default() -> Self {
        Self::new()
    }
}

impl Residue<BackEdgeState> for BackEdgeReducer {
    type Output = BackEdges;

    /// Track a state transition and identify if it's a back-edge.
    ///
    /// A back-edge occurs when we transition from a state to a destination
    /// that appears in the source state's visited path.
    fn new_state(
        &mut self,
        state: &BackEdgeState,
        dest_state: &BackEdgeState,
        _op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        // Extract concrete addresses from both states
        if let (Some(from_addr), Some(to_addr)) = (state.get_location(), dest_state.get_location())
        {
            // Record this edge
            if !self.visited_edges.contains(&(from_addr, to_addr)) {
                self.visited_edges.push((from_addr, to_addr));
            }

            // Check if this is a back-edge:
            // The destination is a back-edge if it appears in the source state's visited path
            if state.path_visits.contains(&dest_state.location) {
                self.back_edges.add(from_addr, to_addr);
            }
        }
    }

    fn new() -> Self {
        Self::new()
    }

    fn finalize(self) -> Self::Output {
        self.back_edges
    }
}

pub struct BackEdgeCPA;

impl Default for BackEdgeCPA {
    fn default() -> Self {
        Self::new()
    }
}

impl BackEdgeCPA {
    pub fn new() -> Self {
        Self
    }

    /// Inherent constructor for the analysis initial state.
    ///
    /// The `Analysis` trait no longer provides an associated `Input` or
    /// `make_initial_state` method. Callers that previously relied on
    /// `analysis.make_initial_state(addr)` can now use
    /// `analysis.make_initial_state(addr)` as an inherent method on the
    /// concrete analysis type.
    pub fn make_initial_state(&self, addr: ConcretePcodeAddress) -> BackEdgeState {
        BackEdgeState::new(PcodeAddressLattice::Const(addr))
    }
}

impl ConfigurableProgramAnalysis for BackEdgeCPA {
    type State = BackEdgeState;
    type Reducer = BackEdgeReducer;
}

pub type BackEdgeAnalysis = BackEdgeCPA;
