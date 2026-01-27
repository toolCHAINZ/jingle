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
            state::{AbstractState, LocationState, MergeOutcome, StateDisplay, Successor},
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
    location: Option<ConcretePcodeAddress>,
    /// Set of visited locations in the current path
    visited: HashSet<ConcretePcodeAddress>,
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
        write!(f, "BackEdgeCount(loc: {:?}, edges: {{", self.location)?;

        // Sort the back-edge counts for deterministic output
        let mut edges: Vec<_> = self.back_edge_counts.iter().collect();
        edges.sort_by_key(|(edge, _)| *edge);

        for (i, (edge, count)) in edges.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "({:?} -> {:?}): {}", edge.0, edge.1, count)?;
        }

        write!(f, "}})")
    }
}

impl LowerHex for UnwindingState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BackEdgeCount(loc: ")?;

        if let Some(loc) = &self.location {
            write!(f, "{:#x}", loc)?;
        } else {
            write!(f, "None")?;
        }

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
        let mut visited = HashSet::new();
        visited.insert(location);
        Self {
            location: Some(location),
            visited,
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
    fn move_to<L: LocationState>(&self, other: &L) -> Option<Self> {
        let new_location = other.get_location()?;

        // Check if this is a back-edge (new_location is already in visited set)
        if self.visited.contains(&new_location) {
            let mut new_state = self.clone();
            if let Some(from) = self.location {
                let edge = (from, new_location);
                *new_state.back_edge_counts.entry(edge).or_insert(0) += 1;
            }

            // Update visited set and location
            new_state.visited.insert(new_location);
            new_state.location = Some(new_location);
            Some(new_state)
        } else {
            None
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
        // Join by taking the maximum count for each back-edge
        for (edge, &count) in &other.back_edge_counts {
            let entry = self.back_edge_counts.entry(*edge).or_insert(0);
            *entry = (*entry).max(count);
        }
        // Merge visited sets
        self.visited.extend(&other.visited);
    }
}

impl StateDisplay for UnwindingState {
    fn fmt_state(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BackEdgeCount({:?}, counts: {:?})",
            self.location, self.back_edge_counts
        )
    }
}

impl AbstractState for UnwindingState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
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
