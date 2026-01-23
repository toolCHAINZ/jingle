// filepath: /Users/denhomc1/RustroverProjects/jingle/jingle/src/analysis/unwinding2.rs

//! A new `Unwinding` analysis using the CPA traits and Compound Analysis framework.
//!
//! # Design Overview
//!
//! This module implements an unwinding analysis that bounds loop iterations by tracking
//! back-edge visits. The design follows these principles:
//!
//! ## Components
//!
//! 1. **BackEdgeCountState** (private sub-analysis):
//!    - Tracks visited locations in the current path
//!    - Maintains a map of back-edge visit counts
//!    - Stores a maximum count threshold
//!    - Incrementing happens via the `Strengthen` trait
//!
//! 2. **BackEdgeCountCPA**:
//!    - The CPA for the back-edge counting state
//!    - Configured with a maximum visit count
//!
//! 3. **Unwinding<L>**:
//!    - A wrapper around a compound analysis `(BackEdgeCountCPA, L)`
//!    - `L` must be a CPA whose state implements `LocationState`
//!    - The location analysis strengthens the back-edge analysis to be location-sensitive
//!
//! ## How It Works
//!
//! The unwinding analysis is a Compound Analysis where:
//! - The `BackEdgeCountState` is strengthened by any `LocationState`
//! - When the location changes, `BackEdgeCountState::strengthen()` is called
//! - The strengthen implementation updates the location and checks for back-edges
//! - A back-edge is detected when we visit a location already in the path's visited set
//! - The count for that back-edge is incremented
//! - When any back-edge count hits the max, `terminated()` returns true
//! - The `transfer` function returns no successors when terminated
//!
//! # Usage
//!
//! ## Direct Construction
//!
//! ```ignore
//! use jingle::analysis::unwinding2::Unwinding;
//! use jingle::analysis::direct_location::DirectLocationAnalysis;
//!
//! let location_analysis = DirectLocationAnalysis::new(...);
//! let unwinding_analysis = Unwinding::new(location_analysis, 5);
//! // This bounds all back-edges to at most 5 iterations
//! ```
//!
//! ## Using the Extension Trait
//!
//! ```ignore
//! use jingle::analysis::unwinding2::UnwindExt;
//! use jingle::analysis::direct_location::DirectLocationAnalysis;
//!
//! let analysis = DirectLocationAnalysis::new(...)
//!     .unwind(10);  // Automatically wraps in Unwinding with bound of 10
//! ```
//!
//! ## Adding Support for Custom CPAs
//!
//! To use `Unwinding` with a custom CPA, you need to implement `CompoundAnalysis`:
//!
//! ```ignore
//! use jingle::analysis::compound::CompoundAnalysis;
//! use jingle::analysis::unwinding2::BackEdgeCountCPA;
//!
//! impl CompoundAnalysis<MyCPA> for BackEdgeCountCPA {}
//! ```
//!
//! This is required because the compound framework cannot implement it generically
//! due to conflicts with blanket implementations for nested compound analyses.

use crate::analysis::Analysis;
use crate::analysis::cfg::CfgState;
use crate::analysis::compound::{CompoundAnalysis, CompoundState, Strengthen, StrengthenOutcome};
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::{
    AbstractState, LocationState, MergeOutcome, StateDisplay, Successor,
};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter, LowerHex, Result as FmtResult};
use std::hash::{Hash, Hasher};

/// A back-edge is a pair of (from, to) addresses
type BackEdge = (ConcretePcodeAddress, ConcretePcodeAddress);

/// Internal state for tracking back-edge visits.
/// This is the "sub" analysis that keeps track of visited locations and back-edge counts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackEdgeCountState {
    /// Current location
    location: Option<ConcretePcodeAddress>,
    /// Set of visited locations in the current path
    visited: HashSet<ConcretePcodeAddress>,
    /// Map of back-edge to visit count
    back_edge_counts: HashMap<BackEdge, usize>,
    /// Maximum allowed visits for any back-edge
    max_count: usize,
}

impl Hash for BackEdgeCountState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut v: Vec<_> = self.back_edge_counts.iter().collect();
        v.sort_by(|a, b| a.0.partial_cmp(b.0).unwrap_or(a.1.cmp(b.1)));
        v.hash(state);
    }
}

impl Display for BackEdgeCountState {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
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

impl LowerHex for BackEdgeCountState {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
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

impl BackEdgeCountState {
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
    fn move_to(&self, new_location: ConcretePcodeAddress) -> Self {
        let mut new_state = self.clone();

        // Check if this is a back-edge (new_location is already in visited set)
        if self.visited.contains(&new_location) {
            if let Some(from) = self.location {
                let edge = (from, new_location);
                *new_state.back_edge_counts.entry(edge).or_insert(0) += 1;
            }
        }

        // Update visited set and location
        new_state.visited.insert(new_location);
        new_state.location = Some(new_location);
        new_state
    }
}

impl PartialOrd for BackEdgeCountState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.location == other.location && self.back_edge_counts == other.back_edge_counts {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

impl JoinSemiLattice for BackEdgeCountState {
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

impl StateDisplay for BackEdgeCountState {
    fn fmt_state(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "BackEdgeCount({:?}, counts: {:?})",
            self.location, self.back_edge_counts
        )
    }
}

impl AbstractState for BackEdgeCountState {
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

/// Internal CPA for back-edge counting
pub struct BackEdgeCountCPA {
    max_count: usize,
}

impl BackEdgeCountCPA {
    pub fn new(max_count: usize) -> Self {
        Self { max_count }
    }
}

impl ConfigurableProgramAnalysis for BackEdgeCountCPA {
    type State = BackEdgeCountState;
    type Reducer = EmptyBackEdgeReducer;
}

/// Empty reducer for the back-edge count CPA
pub struct EmptyBackEdgeReducer;

impl Residue<BackEdgeCountState> for EmptyBackEdgeReducer {
    type Output = ();

    fn new() -> Self {
        EmptyBackEdgeReducer
    }

    fn finalize(self) -> Self::Output {}
}

impl IntoState<BackEdgeCountCPA> for ConcretePcodeAddress {
    fn into_state(self, c: &BackEdgeCountCPA) -> BackEdgeCountState {
        BackEdgeCountState::with_location(self, c.max_count)
    }
}

/// Strengthen implementation: BackEdgeCountState gets strengthened by any LocationState
impl<L: LocationState> Strengthen<L> for BackEdgeCountState {
    fn strengthen(
        &mut self,
        _original: &CompoundState<Self, L>,
        other: &L,
        _op: &PcodeOperation,
    ) -> StrengthenOutcome {
        // Update our location based on the location analysis
        if let Some(new_loc) = other.get_location() {
            if self.location != Some(new_loc) {
                *self = self.move_to(new_loc);
                return StrengthenOutcome::Changed;
            }
        }
        StrengthenOutcome::Unchanged
    }
}

// Implement CompoundAnalysis for BackEdgeCountCPA with common CPA types.
// This allows those CPAs to be used with the Unwinding analysis.

/// Enable BackEdgeCountCPA to be compounded with DirectLocationAnalysis
impl CompoundAnalysis<crate::analysis::direct_location::DirectLocationAnalysis>
    for BackEdgeCountCPA
{
}

/// Enable BackEdgeCountCPA to be compounded with BackEdgeCPA
impl CompoundAnalysis<crate::analysis::back_edge::BackEdgeCPA> for BackEdgeCountCPA {}

/// The main Unwinding analysis.
/// This wraps a tuple-based compound analysis combining back-edge counting with a location analysis.
pub type Unwinding<L> = (BackEdgeCountCPA, L);

impl<L: ConfigurableProgramAnalysis> Analysis for Unwinding<L>
where
    L::State: LocationState,
    BackEdgeCountCPA: CompoundAnalysis<L>,
{
}

/// Extension trait to add `unwind` method to any location analysis
pub trait UnwindExt: ConfigurableProgramAnalysis
where
    Self::State: LocationState,
    BackEdgeCountCPA: CompoundAnalysis<Self>,
{
    /// Wrap this analysis in an Unwinding analysis with the given bound.
    ///
    /// # Arguments
    /// * `bound` - Maximum number of times any back-edge can be traversed
    fn unwind(self, bound: usize) -> (BackEdgeCountCPA, Self)
    where
        Self: Sized,
    {
        (BackEdgeCountCPA::new(bound), self)
    }
}

/// Blanket implementation: any CPA with LocationState and CompoundAnalysis support can be unwound
impl<T> UnwindExt for T
where
    T: ConfigurableProgramAnalysis,
    T::State: LocationState,
    BackEdgeCountCPA: CompoundAnalysis<T>,
{
}

impl<L: LocationState> LocationState for CompoundState<BackEdgeCountState, L> {
    fn get_operation<'a, T: crate::analysis::pcode_store::PcodeStore + ?Sized>(
        &'a self,
        t: &'a T,
    ) -> Option<crate::analysis::pcode_store::PcodeOpRef<'a>> {
        self.1.get_operation(t)
    }

    fn get_location(&self) -> Option<ConcretePcodeAddress> {
        self.1.get_location()
    }
}

impl<L: CfgState> CfgState for CompoundState<BackEdgeCountState, L> {
    type Model = L::Model;

    fn new_const(&self, _i: &SleighArchInfo) -> Self::Model {
        todo!()
    }

    fn model_id(&self) -> String {
        todo!()
    }

    fn location(&self) -> Option<ConcretePcodeAddress> {
        todo!()
    }
}
