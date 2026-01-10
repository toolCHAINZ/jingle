use crate::analysis::Analysis;
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

pub type BackEdge = (ConcretePcodeAddress, ConcretePcodeAddress);

#[derive(Clone, Debug, Default)]
pub struct BackEdges {
    // todo: make generic?
    edges: HashMap<ConcretePcodeAddress, HashSet<ConcretePcodeAddress>>,
}

impl BackEdges {
    pub fn has(&self, from: &ConcretePcodeAddress, to: &ConcretePcodeAddress) -> bool {
        self.edges.get(from).is_some_and(|s| s.contains(to))
    }

    pub fn add(&mut self, from: ConcretePcodeAddress, to: ConcretePcodeAddress) {
        self.edges.entry(from).or_default().insert(to);
    }

    pub fn get_all_for(
        &self,
        from: &ConcretePcodeAddress,
    ) -> Option<HashSet<ConcretePcodeAddress>> {
        self.edges.get(from).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = BackEdge> {
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

impl BackEdgeState {
    pub fn new(location: PcodeAddressLattice) -> BackEdgeState {
        Self {
            location,
            path_visits: Default::default(),
        }
    }
    pub fn add_location(&self, loc: PcodeAddressLattice) -> BackEdgeState {
        let mut s = self.clone();
        s.location = loc;
        s.path_visits.insert(loc);
        s
    }
}

impl From<ConcretePcodeAddress> for BackEdgeState {
    fn from(addr: ConcretePcodeAddress) -> Self {
        Self::new(PcodeAddressLattice::Value(addr))
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
    fn get_operation<T: PcodeStore>(&self, t: &T) -> Option<PcodeOperation> {
        self.location.get_operation(t)
    }
}

pub struct BackEdgeCPA {
    pub back_edges: Vec<(PcodeAddressLattice, PcodeAddressLattice)>,
}

impl Default for BackEdgeCPA {
    fn default() -> Self {
        Self::new()
    }
}

impl BackEdgeCPA {
    pub fn new() -> Self {
        Self {
            back_edges: Vec::new(),
        }
    }

    /// Extract the computed back edges into a BackEdges structure
    pub fn get_back_edges(&self) -> BackEdges {
        let mut b = BackEdges::default();
        for (from, to) in &self.back_edges {
            if let (PcodeAddressLattice::Value(from), PcodeAddressLattice::Value(to)) = (from, to) {
                b.add(*from, *to);
            }
        }
        b
    }

    /// Inherent constructor for the analysis initial state.
    ///
    /// The `Analysis` trait no longer provides an associated `Input` or
    /// `make_initial_state` method. Callers that previously relied on
    /// `analysis.make_initial_state(addr)` can now use
    /// `analysis.make_initial_state(addr)` as an inherent method on the
    /// concrete analysis type.
    pub fn make_initial_state(&self, addr: ConcretePcodeAddress) -> BackEdgeState {
        BackEdgeState::new(PcodeAddressLattice::Value(addr))
    }
}

impl ConfigurableProgramAnalysis for BackEdgeCPA {
    type State = BackEdgeState;

    fn reduce(
        &mut self,
        old_state: &Self::State,
        new_state: &Self::State,
        _op: &Option<PcodeOperation>,
    ) {
        if old_state.path_visits.contains(&new_state.location) {
            self.back_edges
                .push((old_state.location, new_state.location))
        }
    }
}

impl Analysis for BackEdgeCPA {}

pub type BackEdgeAnalysis = BackEdgeCPA;
