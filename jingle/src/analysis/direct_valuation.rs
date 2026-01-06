//! Direct Valuation Analysis
//!
//! This module provides a Configurable Program Analysis for tracking direct writes to varnodes.
//! The state tracks all written memory locations at each code location.
//!
//! The analysis follows these rules:
//! - PcodeOperation::Copy from CONST_SPACE_INDEX to any other varnode: track the written location
//! - Branch to a different machine address: clear all varnodes in "internal" space types
//! - Any other write to a varnode: set it to Top
//!
//! # Example
//!
//! ```ignore
//! use jingle::analysis::{Analysis, RunnableAnalysis};
//! use jingle::analysis::direct_location::DirectLocationAnalysis;
//! use jingle::analysis::direct_valuation::DirectValuationAnalysis;
//!
//! // Create a compound analysis: location tracking + direct valuation
//! let location_analysis = DirectLocationAnalysis::new(&loaded);
//! let valuation_analysis = DirectValuationAnalysis;
//!
//! let mut compound_analysis = (location_analysis, valuation_analysis);
//!
//! // Run the analysis
//! let states = compound_analysis.run(&loaded, compound_analysis.make_initial_state(entry_addr.into()));
//!
//! // Extract results
//! for state in &states {
//!     if let FlatLattice::Value(addr) = &state.left {
//!         println!("At {:x}:", addr);
//!         for (varnode, value) in state.right.written_locations() {
//!             println!("  {:?} = {:?}", varnode, value);
//!         }
//!     }
//! }
//! ```

use crate::analysis::Analysis;
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{GeneralizedVarNode, PcodeOperation, VarNode};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashMap;

/// Represents the value state of a varnode
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum VarnodeValue {
    /// No information (bottom of lattice)
    Bottom,
    /// A direct constant write
    DirectConstant(u64),
    /// Unknown value (top of lattice)
    Top,
}

impl PartialOrd for VarnodeValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use VarnodeValue::*;
        match (self, other) {
            // Bottom is less than everything except itself
            (Bottom, Bottom) => Some(Ordering::Equal),
            (Bottom, _) => Some(Ordering::Less),
            (_, Bottom) => Some(Ordering::Greater),

            // Top is greater than everything except itself
            (Top, Top) => Some(Ordering::Equal),
            (Top, _) => Some(Ordering::Greater),
            (_, Top) => Some(Ordering::Less),

            // Equal constants
            (DirectConstant(a), DirectConstant(b)) if a == b => Some(Ordering::Equal),

            // Incomparable constants
            _ => None,
        }
    }
}

impl JoinSemiLattice for VarnodeValue {
    fn join(&mut self, other: &Self) {
        use VarnodeValue::*;
        *self = match (&*self, other) {
            (Bottom, x) | (x, Bottom) => x.clone(),
            (Top, _) | (_, Top) => Top,
            (DirectConstant(a), DirectConstant(b)) if a == b => DirectConstant(*a),
            (DirectConstant(_), DirectConstant(_)) => Top,
        };
    }
}

/// Abstract state for direct valuation analysis
///
/// This state tracks the values of varnodes that have been written directly from constants.
/// The map contains all known written locations at a given code location.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DirectValuationState {
    /// Map of written varnodes to their values
    written_locations: HashMap<VarNode, VarnodeValue>,
}

impl DirectValuationState {
    /// Create a new empty direct valuation state
    pub fn new() -> Self {
        Self {
            written_locations: HashMap::new(),
        }
    }

    /// Get the value of a written varnode
    pub fn get_value(&self, varnode: &VarNode) -> Option<&VarnodeValue> {
        self.written_locations.get(varnode)
    }

    /// Get all written locations
    pub fn written_locations(&self) -> &HashMap<VarNode, VarnodeValue> {
        &self.written_locations
    }

    /// Transfer function for direct valuation analysis
    fn transfer_impl(&self, op: &PcodeOperation, current_location: Option<ConcretePcodeAddress>) -> Self {
        let mut new_state = self.clone();

        // Check if this is a branch to a different machine address
        let is_cross_machine_branch = match op {
            PcodeOperation::Branch { input } => {
                // Absolute branch to different machine address
                if let Some(loc) = current_location {
                    input.offset != loc.machine()
                } else {
                    false
                }
            }
            PcodeOperation::Call { dest, .. } => {
                // Call to different machine address
                if let Some(loc) = current_location {
                    dest.offset != loc.machine()
                } else {
                    false
                }
            }
            PcodeOperation::CBranch { input0, .. } => {
                // Conditional branch - check if target is different machine address
                if let Some(loc) = current_location {
                    !input0.is_const() && input0.offset != loc.machine()
                } else {
                    false
                }
            }
            _ => false,
        };

        // If we're branching to a different machine address, clear internal space varnodes
        if is_cross_machine_branch {
            // We'll need SpaceInfo to check space types, but for now we can't access it here
            // This will be addressed in the strengthen method or by passing additional context
            // For now, we'll keep all entries (this is a limitation that should be noted)
            // TODO: Clear varnodes in internal spaces when we have access to SpaceInfo
        }

        // Handle writes
        if let Some(output) = op.output() {
            match output {
                GeneralizedVarNode::Direct(output_vn) => {
                    match op {
                        PcodeOperation::Copy { input, .. } => {
                            // Track direct constant writes
                            if input.space_index == VarNode::CONST_SPACE_INDEX {
                                new_state.written_locations.insert(
                                    output_vn,
                                    VarnodeValue::DirectConstant(input.offset),
                                );
                            } else {
                                // Copy from non-constant: set to Top
                                new_state.written_locations.insert(output_vn, VarnodeValue::Top);
                            }
                        }
                        _ => {
                            // Any other write: set to Top
                            new_state.written_locations.insert(output_vn, VarnodeValue::Top);
                        }
                    }
                }
                GeneralizedVarNode::Indirect(_) => {
                    // Indirect writes are not tracked for now
                    // Could potentially set all aliasing locations to Top
                }
            }
        }

        new_state
    }
}

impl Default for DirectValuationState {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialOrd for DirectValuationState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Two states are comparable if all their entries are comparable
        // and they have the same keys
        if self.written_locations.len() != other.written_locations.len() {
            return None;
        }

        let mut overall = Ordering::Equal;
        for (key, value) in &self.written_locations {
            match other.written_locations.get(key) {
                Some(other_value) => match value.partial_cmp(other_value) {
                    Some(Ordering::Equal) => continue,
                    Some(Ordering::Less) => {
                        if overall == Ordering::Greater {
                            return None;
                        }
                        overall = Ordering::Less;
                    }
                    Some(Ordering::Greater) => {
                        if overall == Ordering::Less {
                            return None;
                        }
                        overall = Ordering::Greater;
                    }
                    None => return None,
                },
                None => return None,
            }
        }

        Some(overall)
    }
}

impl JoinSemiLattice for DirectValuationState {
    fn join(&mut self, other: &Self) {
        // Join the maps
        for (key, other_value) in &other.written_locations {
            self.written_locations
                .entry(key.clone())
                .and_modify(|v| v.join(other_value))
                .or_insert_with(|| other_value.clone());
        }
    }
}

impl AbstractState for DirectValuationState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_join(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, opcode: B) -> Successor<'a, Self> {
        let next_state = self.transfer_impl(opcode.borrow(), None);
        std::iter::once(next_state).into()
    }
}

// Strengthen implementations for compound analysis
impl crate::analysis::compound::Strengthen<crate::analysis::cpa::lattice::pcode::PcodeAddressLattice> for DirectValuationState {
    fn strengthen(&mut self, original: &Self, location: &crate::analysis::cpa::lattice::pcode::PcodeAddressLattice, op: &jingle_sleigh::PcodeOperation) -> crate::analysis::compound::StrengthenOutcome {
        use crate::analysis::cpa::lattice::flat::FlatLattice;
        
        // When we have location information and this is a branch to a different machine address,
        // we can clear internal space varnodes
        
        // Check if this is a cross-machine branch
        if let FlatLattice::Value(addr) = location {
            let is_cross_machine_branch = match op {
                jingle_sleigh::PcodeOperation::Branch { input } => {
                    !input.is_const() && input.offset != addr.machine()
                }
                jingle_sleigh::PcodeOperation::Call { dest, .. } => {
                    dest.offset != addr.machine()
                }
                jingle_sleigh::PcodeOperation::CBranch { input0, .. } => {
                    !input0.is_const() && input0.offset != addr.machine()
                }
                _ => false,
            };
            
            if is_cross_machine_branch {
                // Clear varnodes in internal spaces
                // Note: We don't have access to SpaceInfo here to check space types,
                // so this is a placeholder for future enhancement
                // In practice, you would need to pass SpaceInfo through the analysis
                // or store it in the state
                
                // For now, just return Unchanged as we can't determine space types
                // TODO: Add SpaceInfo access to properly clear internal space varnodes
            }
        }
        
        crate::analysis::compound::StrengthenOutcome::Unchanged
    }
}

impl crate::analysis::compound::Strengthen<crate::analysis::unwinding::UnwindingCpaState> for DirectValuationState {}

// Make DirectLocationAnalysis compatible with DirectValuationAnalysis in compound analysis
impl crate::analysis::compound::CompoundAnalysis<DirectValuationAnalysis> for crate::analysis::direct_location::DirectLocationAnalysis {}

/// The Direct Valuation CPA
pub struct DirectValuationAnalysis;

impl ConfigurableProgramAnalysis for DirectValuationAnalysis {
    type State = DirectValuationState;
}

impl Analysis for DirectValuationAnalysis {
    type Input = DirectValuationState;

    fn make_initial_state(&self, _addr: ConcretePcodeAddress) -> Self::Input {
        DirectValuationState::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varnode_value_ordering() {
        let bottom = VarnodeValue::Bottom;
        let const_10 = VarnodeValue::DirectConstant(10);
        let const_20 = VarnodeValue::DirectConstant(20);
        let top = VarnodeValue::Top;

        assert!(bottom < const_10);
        assert!(bottom < top);
        assert!(const_10 < top);
        assert!(const_10.partial_cmp(&const_20).is_none());
    }

    #[test]
    fn test_varnode_value_join() {
        let mut val1 = VarnodeValue::DirectConstant(10);
        let val2 = VarnodeValue::DirectConstant(10);
        val1.join(&val2);
        assert_eq!(val1, VarnodeValue::DirectConstant(10));

        let mut val1 = VarnodeValue::DirectConstant(10);
        let val2 = VarnodeValue::DirectConstant(20);
        val1.join(&val2);
        assert_eq!(val1, VarnodeValue::Top);
    }

    #[test]
    fn test_copy_from_constant() {
        let state = DirectValuationState::new();
        let output = VarNode {
            space_index: 1,
            offset: 100,
            size: 8,
        };
        let input = VarNode {
            space_index: VarNode::CONST_SPACE_INDEX,
            offset: 42,
            size: 8,
        };
        let op = PcodeOperation::Copy {
            input: input.clone(),
            output: output.clone(),
        };

        let new_state = state.transfer_impl(&op, None);
        assert_eq!(
            new_state.get_value(&output),
            Some(&VarnodeValue::DirectConstant(42))
        );
    }

    #[test]
    fn test_copy_from_non_constant() {
        let state = DirectValuationState::new();
        let output = VarNode {
            space_index: 1,
            offset: 100,
            size: 8,
        };
        let input = VarNode {
            space_index: 2,
            offset: 200,
            size: 8,
        };
        let op = PcodeOperation::Copy {
            input: input.clone(),
            output: output.clone(),
        };

        let new_state = state.transfer_impl(&op, None);
        assert_eq!(new_state.get_value(&output), Some(&VarnodeValue::Top));
    }
}

