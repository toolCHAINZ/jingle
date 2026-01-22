use crate::analysis::back_edge::{BackEdge, BackEdges};
use crate::analysis::cfg::CfgState;
use crate::analysis::cpa::lattice::{JoinSemiLattice, PartialJoinSemiLattice};
use crate::analysis::cpa::state::{
    AbstractState, LocationState, MergeOutcome, StateDisplay, Successor,
};
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Formatter, Result};
use std::hash::{Hash, Hasher};
use std::iter::empty;

#[derive(Debug, Clone, Eq)]
pub struct BackEdgeVisitCountState<L: LocationState> {
    pub location: L,
    pub back_edge_visits: HashMap<(ConcretePcodeAddress, ConcretePcodeAddress), usize>,
    pub max: usize,
}

impl<L: LocationState + Hash> Hash for BackEdgeVisitCountState<L> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.location.hash(state);
        // Hash the visit counts in a deterministic order
        let mut visits: Vec<_> = self.back_edge_visits.iter().collect();
        visits.sort_by_key(|(k, _)| *k);
        for (edge, count) in visits {
            edge.hash(state);
            count.hash(state);
        }
        self.max.hash(state);
    }
}

impl<L: LocationState> BackEdgeVisitCountState<L> {
    pub fn new(location: L, back_edges: BackEdges, max: usize) -> Self {
        BackEdgeVisitCountState {
            location,
            back_edge_visits: back_edges.iter().map(|k| (k, 0)).collect(),
            max,
        }
    }

    pub fn back_edge_str(&self) -> Vec<usize> {
        let mut sorted = self
            .back_edge_visits
            .clone()
            .into_iter()
            .collect::<Vec<(BackEdge, usize)>>();
        sorted.sort_by(|(a, _), (b, _)| match a.0.cmp(&b.0) {
            Ordering::Equal => a.1.cmp(&b.1),
            a => a,
        });
        let strs: Vec<_> = sorted.into_iter().map(|(_, size)| size).collect();
        strs
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

    pub fn same_visit_counts(&self, other: &Self) -> bool {
        self.back_edge_visits.eq(&other.back_edge_visits)
    }

    pub fn max(&self) -> usize {
        self.max
    }
}

impl<L: LocationState> PartialEq for BackEdgeVisitCountState<L> {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location && self.same_visit_counts(other)
    }
}

impl<L: LocationState> PartialOrd for BackEdgeVisitCountState<L> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.location == other.location && self.same_visit_counts(other) {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

impl<L: LocationState> PartialJoinSemiLattice for BackEdgeVisitCountState<L> {
    fn partial_join(&self, other: &Self) -> Option<Self> {
        let mut visits = HashMap::new();
        for (addr, count) in self.back_edge_visits.iter() {
            let count = *count;
            let max: usize = count.max(other.back_edge_visits.get(addr).cloned().unwrap_or(0));
            visits.insert(*addr, max);
        }
        let s = Self {
            location: self.location.clone(),
            back_edge_visits: visits,
            max: self.max,
        };
        Some(s)
    }
}

impl<L: LocationState> JoinSemiLattice for BackEdgeVisitCountState<L> {
    fn join(&mut self, other: &Self) {
        for (addr, count) in self.back_edge_visits.iter_mut() {
            let max: usize = other.back_edge_visits.get(addr).cloned().unwrap_or(0);
            *count = max;
        }
    }
}

impl<L: LocationState> StateDisplay for BackEdgeVisitCountState<L> {
    fn fmt_state(&self, f: &mut Formatter<'_>) -> Result {
        let counts = self.back_edge_str();
        let formatted = counts
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("_");
        write!(f, "{}", formatted)
    }
}

impl<L: LocationState> AbstractState for BackEdgeVisitCountState<L> {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, opcode: B) -> Successor<'a, Self> {
        // If we've hit the termination condition, stop exploration
        if self.terminated() {
            return empty().into();
        }

        let opcode = opcode.borrow();

        // Get the successors from the inner location state
        let location_successors: Vec<L> = self.location.transfer(opcode).into_iter().collect();

        // For each location successor, create a new BackEdgeVisitCountState
        let mut result = Vec::new();
        for new_location in location_successors {
            let mut new_state = self.clone();

            // Check if this transition is a back-edge and increment the count
            if let (Some(from), Some(to)) =
                (self.location.get_location(), new_location.get_location())
            {
                let edge = (from, to);
                if new_state.back_edge_visits.contains_key(&edge) {
                    new_state.increment_back_edge_count(edge);
                }
            }

            // Update the location
            new_state.location = new_location;
            result.push(new_state);
        }

        result.into_iter().into()
    }
}

impl<L: LocationState> LocationState for BackEdgeVisitCountState<L> {
    fn get_operation<T: PcodeStore>(&self, t: &T) -> Option<&PcodeOperation> {
        self.location.get_operation(t)
    }

    fn get_location(&self) -> Option<ConcretePcodeAddress> {
        self.location.get_location()
    }
}

impl<L> CfgState for BackEdgeVisitCountState<L>
where
    L: CfgState + LocationState + Hash,
{
    type Model = L::Model;

    fn new_const(&self, i: &jingle_sleigh::SleighArchInfo) -> Self::Model {
        self.location.new_const(i)
    }

    fn model_id(&self) -> String {
        // Include the back-edge visit counts in the model ID for uniqueness
        let counts = self.back_edge_str();
        let counts_str = counts
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("_");
        format!("{}_{}", self.location.model_id(), counts_str)
    }

    fn location(&self) -> Option<ConcretePcodeAddress> {
        self.location.location()
    }
}
