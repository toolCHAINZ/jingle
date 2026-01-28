use std::{
    borrow::Borrow,
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt::{Debug, Display, Formatter, LowerHex},
    hash::{Hash, Hasher},
};

use jingle_sleigh::PcodeOperation;

use crate::{
    analysis::{
        cpa::{
            IntoState,
            lattice::JoinSemiLattice,
            state::{AbstractState, LocationState, MergeOutcome, Successor},
        },
        location::{basic::state::BasicLocationState, unwind::UnwindingAnalysis},
    },
    modeling::machine::cpu::concrete::ConcretePcodeAddress,
    register_strengthen,
};

/// A back-edge is a pair of (from, to) addresses
type BackEdge = (ConcretePcodeAddress, ConcretePcodeAddress);

/// Internal state for tracking back-edge visits.
/// This is the "sub" analysis that keeps track of visited locations and back-edge counts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnwindingState {
    /// Current location
    location: ConcretePcodeAddress,
    /// Set of visited locations in the current path
    dominators: Vec<ConcretePcodeAddress>,
    /// Map of back-edge to visit count
    back_edge_counts: HashMap<BackEdge, usize>,
    /// Maximum allowed visits for any back-edge
    max_count: usize,
}

impl Hash for UnwindingState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut v: Vec<_> = self.back_edge_counts.iter().collect();
        v.sort_by(|a, b| a.0.partial_cmp(b.0).unwrap_or(a.1.cmp(b.1)));
        v.hash(state);
    }
}

impl Display for UnwindingState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Sort the back-edge counts for deterministic output
        let mut edges: Vec<_> = self.back_edge_counts.iter().collect();
        edges.sort_by_key(|(edge, _)| *edge);

        for (i, (edge, count)) in edges.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "({:x} -> {:x}):{}", edge.0, edge.1, count)?;
        }
        Ok(())
    }
}

impl LowerHex for UnwindingState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BackEdgeCount(loc: ")?;

        write!(f, "{:#x}", self.location)?;

        write!(f, ", edges: {{")?;

        // Sort the back-edge counts for deterministic output
        let mut edges: Vec<_> = self.back_edge_counts.iter().collect();
        edges.sort_by_key(|(edge, _)| *edge);

        for (i, (edge, count)) in edges.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "({:#x} -> {:#x}): {}", edge.0, edge.1, count)?;
        }

        write!(f, "}})")
    }
}

impl UnwindingState {
    fn with_location(location: ConcretePcodeAddress, max_count: usize) -> Self {
        let mut dominators = vec![location];
        Self {
            location,
            dominators,
            back_edge_counts: HashMap::new(),
            max_count,
        }
    }

    /// Check if this state has hit the termination condition
    fn terminated(&self) -> bool {
        self.back_edge_counts
            .values()
            .any(|&count| count >= self.max_count)
    }

    /// Move to a new location, updating visited set and back-edge counts
    fn move_to<L: LocationState>(&mut self, other: &L) {
        if let Some(new_location) = other.get_location() {
            // Check if this is a back-edge (new_location is already in visited set)
            if let Some(idx) = self.dominators.iter().position(|p| p == &new_location) {
                let edge = (self.location, new_location);
                *self.back_edge_counts.entry(edge).or_insert(0) += 1;
                self.dominators.truncate(idx);
                // Update visited set and location
            } else {
                self.dominators.push(new_location);
            }
            self.location = new_location;
        }
    }
}

impl PartialOrd for UnwindingState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.location == other.location && self.back_edge_counts == other.back_edge_counts {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

impl JoinSemiLattice for UnwindingState {
    fn join(&mut self, other: &Self) {
        // Merge max_count conservatively (choose the larger limit)
        self.max_count = self.max_count.max(other.max_count);

        let uneq_idx = self
            .dominators
            .iter()
            .zip(&other.dominators)
            .position(|(a, b)| a != b);

        if let Some(uneq_idx) = uneq_idx {
            self.dominators.truncate(uneq_idx);
        }

        if self.dominators.last() != Some(&self.location) {
            self.dominators.push(self.location);
        }

        // For back-edge counts, take the maximum count for each edge across both maps.
        for (edge, &other_count) in other.back_edge_counts.iter() {
            let key = edge.clone();
            let entry = self.back_edge_counts.entry(key).or_insert(0);
            if *entry < other_count {
                *entry = other_count;
            }
        }
        // Existing edges in self.back_edge_counts remain as they are (they already represent
        // the maximum with respect to themselves).
    }
}

impl AbstractState for UnwindingState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_join(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, _opcode: B) -> Successor<'a, Self> {
        // If we've hit the termination condition, stop exploration
        if self.terminated() {
            return std::iter::empty().into();
        }

        // This state doesn't transfer on its own - it gets strengthened by the location analysis
        // Return self unchanged
        std::iter::once(self.clone()).into()
    }
}

impl IntoState<UnwindingAnalysis> for ConcretePcodeAddress {
    fn into_state(self, c: &UnwindingAnalysis) -> UnwindingState {
        UnwindingState::with_location(self, c.max_count)
    }
}

register_strengthen!(UnwindingState, BasicLocationState, UnwindingState::move_to);
