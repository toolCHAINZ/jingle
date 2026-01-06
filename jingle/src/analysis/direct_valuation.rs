//! Direct Valuation Analysis
//!
//! This module provides a Configurable Program Analysis that acts as a lightweight pcode interpreter.
//! It tracks the abstract values of all directly-written varnodes through program execution.
//!
//! The analysis uses a four-level lattice for varnode values:
//! - Entry: the original value of a varnode at function entry
//! - Offset: a constant signed offset relative to the entry value
//! - Const: a constant value, not dependent on the entry value
//! - Top: indeterminate/unknown value
//!
//! This analysis is particularly useful for tracking stack offsets, register values, and
//! understanding how constants propagate through the program.
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
//! let valuation_analysis = DirectValuationAnalysis::new(stack_pointer_varnode);
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

/// Represents the abstract value of a varnode in the analysis
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum VarnodeValue {
    /// No information (bottom of lattice)
    Bottom,
    /// The original entry value of this varnode (e.g., stack pointer at function entry)
    Entry(VarNode),
    /// A constant signed offset relative to the entry value
    Offset(VarNode, i64),
    /// A concrete constant value, not dependent on any entry value
    Const(u64),
    /// Unknown/indeterminate value (top of lattice)
    Top,
}

impl VarnodeValue {
    /// Check if this is a constant value
    pub fn is_const(&self) -> bool {
        matches!(self, VarnodeValue::Const(_))
    }

    /// Get the constant value if it exists
    pub fn as_const(&self) -> Option<u64> {
        match self {
            VarnodeValue::Const(v) => Some(*v),
            _ => None,
        }
    }

    /// Check if this is an entry value
    pub fn is_entry(&self) -> bool {
        matches!(self, VarnodeValue::Entry(_))
    }

    /// Check if this is an offset value
    pub fn is_offset(&self) -> bool {
        matches!(self, VarnodeValue::Offset(_, _))
    }

    /// Add a constant to this value
    fn add(&self, delta: i64) -> Self {
        match self {
            VarnodeValue::Bottom => VarnodeValue::Bottom,
            VarnodeValue::Entry(vn) => VarnodeValue::Offset(vn.clone(), delta),
            VarnodeValue::Offset(vn, offset) => VarnodeValue::Offset(vn.clone(), offset.wrapping_add(delta)),
            VarnodeValue::Const(val) => VarnodeValue::Const(val.wrapping_add(delta as u64)),
            VarnodeValue::Top => VarnodeValue::Top,
        }
    }

    /// Subtract a constant from this value
    fn sub(&self, delta: i64) -> Self {
        match self {
            VarnodeValue::Bottom => VarnodeValue::Bottom,
            VarnodeValue::Entry(vn) => VarnodeValue::Offset(vn.clone(), -delta),
            VarnodeValue::Offset(vn, offset) => VarnodeValue::Offset(vn.clone(), offset.wrapping_sub(delta)),
            VarnodeValue::Const(val) => VarnodeValue::Const(val.wrapping_sub(delta as u64)),
            VarnodeValue::Top => VarnodeValue::Top,
        }
    }

    /// Negate this value
    fn negate(&self) -> Self {
        match self {
            VarnodeValue::Bottom => VarnodeValue::Bottom,
            VarnodeValue::Const(val) => VarnodeValue::Const((*val as i64).wrapping_neg() as u64),
            _ => VarnodeValue::Top,
        }
    }

    /// Bitwise AND with another value
    fn and(&self, other: &Self) -> Self {
        match (self, other) {
            (VarnodeValue::Bottom, _) | (_, VarnodeValue::Bottom) => VarnodeValue::Bottom,
            (VarnodeValue::Const(a), VarnodeValue::Const(b)) => VarnodeValue::Const(a & b),
            (VarnodeValue::Top, _) | (_, VarnodeValue::Top) => VarnodeValue::Top,
            _ => VarnodeValue::Top,
        }
    }

    /// Bitwise OR with another value
    fn or(&self, other: &Self) -> Self {
        match (self, other) {
            (VarnodeValue::Bottom, _) | (_, VarnodeValue::Bottom) => VarnodeValue::Bottom,
            (VarnodeValue::Const(a), VarnodeValue::Const(b)) => VarnodeValue::Const(a | b),
            (VarnodeValue::Top, _) | (_, VarnodeValue::Top) => VarnodeValue::Top,
            _ => VarnodeValue::Top,
        }
    }

    /// Bitwise XOR with another value
    fn xor(&self, other: &Self) -> Self {
        match (self, other) {
            (VarnodeValue::Bottom, _) | (_, VarnodeValue::Bottom) => VarnodeValue::Bottom,
            (VarnodeValue::Const(a), VarnodeValue::Const(b)) => VarnodeValue::Const(a ^ b),
            (VarnodeValue::Top, _) | (_, VarnodeValue::Top) => VarnodeValue::Top,
            _ => VarnodeValue::Top,
        }
    }
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

            // Equal entries
            (Entry(a), Entry(b)) if a == b => Some(Ordering::Equal),

            // Entry is less than its offset variants
            (Entry(a), Offset(b, _)) if a == b => Some(Ordering::Less),
            (Offset(a, _), Entry(b)) if a == b => Some(Ordering::Greater),

            // Equal offset variants
            (Offset(a, off_a), Offset(b, off_b)) if a == b && off_a == off_b => Some(Ordering::Equal),

            // Equal constants
            (Const(a), Const(b)) if a == b => Some(Ordering::Equal),

            // Everything else is incomparable
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
            (Entry(a), Entry(b)) if a == b => Entry(a.clone()),
            (Entry(a), Offset(b, off)) | (Offset(b, off), Entry(a)) if a == b => Offset(a.clone(), *off),
            (Offset(a, off_a), Offset(b, off_b)) if a == b && off_a == off_b => Offset(a.clone(), *off_a),
            (Const(a), Const(b)) if a == b => Const(*a),
            _ => Top,
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

    /// Get the value of a varnode, returns Top if not found
    fn get_value_or_top(&self, varnode: &VarNode) -> VarnodeValue {
        self.written_locations
            .get(varnode)
            .cloned()
            .unwrap_or(VarnodeValue::Top)
    }

    /// Extract constant value from a varnode (either from state or const space)
    fn extract_const(&self, varnode: &VarNode) -> Option<u64> {
        if varnode.space_index == VarNode::CONST_SPACE_INDEX {
            Some(varnode.offset)
        } else {
            self.get_value(varnode).and_then(|v| v.as_const())
        }
    }

    /// Transfer function for direct valuation analysis - lightweight pcode interpreter
    fn transfer_impl(&self, op: &PcodeOperation) -> Self {
        let mut new_state = self.clone();

        // Handle writes based on operation
        if let Some(output) = op.output() {
            match output {
                GeneralizedVarNode::Direct(output_vn) => {
                    let result_value = match op {
                        // Copy: preserve the value
                        PcodeOperation::Copy { input, .. } => {
                            if input.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input.offset)
                            } else {
                                self.get_value_or_top(input)
                            }
                        }

                        // Integer arithmetic operations
                        PcodeOperation::IntAdd { input0, input1, .. } => {
                            let val0 = if input0.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input0.offset)
                            } else {
                                self.get_value_or_top(input0)
                            };
                            let val1 = if input1.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input1.offset)
                            } else {
                                self.get_value_or_top(input1)
                            };

                            match (val0, val1) {
                                (VarnodeValue::Const(a), VarnodeValue::Const(b)) => {
                                    VarnodeValue::Const(a.wrapping_add(b))
                                }
                                (val, VarnodeValue::Const(c)) | (VarnodeValue::Const(c), val) => {
                                    val.add(c as i64)
                                }
                                _ => VarnodeValue::Top,
                            }
                        }

                        PcodeOperation::IntSub { input0, input1, .. } => {
                            let val0 = if input0.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input0.offset)
                            } else {
                                self.get_value_or_top(input0)
                            };
                            let val1 = if input1.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input1.offset)
                            } else {
                                self.get_value_or_top(input1)
                            };

                            match (val0, val1) {
                                (VarnodeValue::Const(a), VarnodeValue::Const(b)) => {
                                    VarnodeValue::Const(a.wrapping_sub(b))
                                }
                                (val, VarnodeValue::Const(c)) => val.sub(c as i64),
                                // Subtracting entry from entry gives 0 offset
                                (VarnodeValue::Entry(a), VarnodeValue::Entry(b)) if a == b => {
                                    VarnodeValue::Const(0)
                                }
                                (VarnodeValue::Offset(a, off_a), VarnodeValue::Entry(b)) if a == b => {
                                    VarnodeValue::Const(off_a as u64)
                                }
                                (VarnodeValue::Offset(a, off_a), VarnodeValue::Offset(b, off_b)) if a == b => {
                                    VarnodeValue::Const((off_a - off_b) as u64)
                                }
                                _ => VarnodeValue::Top,
                            }
                        }

                        PcodeOperation::IntMult { input0, input1, .. } => {
                            if let (Some(a), Some(b)) = (self.extract_const(input0), self.extract_const(input1)) {
                                VarnodeValue::Const(a.wrapping_mul(b))
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::IntDiv { input0, input1, .. } => {
                            if let (Some(a), Some(b)) = (self.extract_const(input0), self.extract_const(input1)) {
                                if b != 0 {
                                    VarnodeValue::Const(a.wrapping_div(b))
                                } else {
                                    VarnodeValue::Top
                                }
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::IntSignedDiv { input0, input1, .. } => {
                            if let (Some(a), Some(b)) = (self.extract_const(input0), self.extract_const(input1)) {
                                if b != 0 {
                                    VarnodeValue::Const((a as i64).wrapping_div(b as i64) as u64)
                                } else {
                                    VarnodeValue::Top
                                }
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::IntRem { input0, input1, .. } => {
                            if let (Some(a), Some(b)) = (self.extract_const(input0), self.extract_const(input1)) {
                                if b != 0 {
                                    VarnodeValue::Const(a.wrapping_rem(b))
                                } else {
                                    VarnodeValue::Top
                                }
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::IntSignedRem { input0, input1, .. } => {
                            if let (Some(a), Some(b)) = (self.extract_const(input0), self.extract_const(input1)) {
                                if b != 0 {
                                    VarnodeValue::Const((a as i64).wrapping_rem(b as i64) as u64)
                                } else {
                                    VarnodeValue::Top
                                }
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::IntNegate { input, .. } => {
                            if input.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input.offset)
                            } else {
                                self.get_value_or_top(input).negate()
                            }
                        }

                        PcodeOperation::Int2Comp { input, .. } => {
                            if input.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const((!(input.offset as i64)) as u64)
                            } else if let Some(c) = self.extract_const(input) {
                                VarnodeValue::Const(!c)
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        // Bitwise operations
                        PcodeOperation::IntAnd { input0, input1, .. } => {
                            let val0 = if input0.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input0.offset)
                            } else {
                                self.get_value_or_top(input0)
                            };
                            let val1 = if input1.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input1.offset)
                            } else {
                                self.get_value_or_top(input1)
                            };
                            val0.and(&val1)
                        }

                        PcodeOperation::IntOr { input0, input1, .. } => {
                            let val0 = if input0.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input0.offset)
                            } else {
                                self.get_value_or_top(input0)
                            };
                            let val1 = if input1.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input1.offset)
                            } else {
                                self.get_value_or_top(input1)
                            };
                            val0.or(&val1)
                        }

                        PcodeOperation::IntXor { input0, input1, .. } => {
                            let val0 = if input0.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input0.offset)
                            } else {
                                self.get_value_or_top(input0)
                            };
                            let val1 = if input1.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input1.offset)
                            } else {
                                self.get_value_or_top(input1)
                            };
                            val0.xor(&val1)
                        }

                        PcodeOperation::IntLeftShift { input0, input1, .. } => {
                            if let (Some(a), Some(b)) = (self.extract_const(input0), self.extract_const(input1)) {
                                VarnodeValue::Const(a.wrapping_shl(b as u32))
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::IntRightShift { input0, input1, .. } => {
                            if let (Some(a), Some(b)) = (self.extract_const(input0), self.extract_const(input1)) {
                                VarnodeValue::Const(a.wrapping_shr(b as u32))
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::IntSignedRightShift { input0, input1, .. } => {
                            if let (Some(a), Some(b)) = (self.extract_const(input0), self.extract_const(input1)) {
                                VarnodeValue::Const((a as i64).wrapping_shr(b as u32) as u64)
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        // Sign/Zero extension - preserve constants
                        PcodeOperation::IntSExt { input, .. } | PcodeOperation::IntZExt { input, .. } => {
                            if input.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input.offset)
                            } else {
                                self.get_value_or_top(input)
                            }
                        }

                        // Comparison operations - we can't track these precisely, so Top
                        PcodeOperation::IntEqual { .. }
                        | PcodeOperation::IntNotEqual { .. }
                        | PcodeOperation::IntLess { .. }
                        | PcodeOperation::IntLessEqual { .. }
                        | PcodeOperation::IntSignedLess { .. }
                        | PcodeOperation::IntSignedLessEqual { .. } => VarnodeValue::Top,

                        // Boolean operations
                        PcodeOperation::BoolAnd { input0, input1, .. } => {
                            let val0 = if input0.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input0.offset)
                            } else {
                                self.get_value_or_top(input0)
                            };
                            let val1 = if input1.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input1.offset)
                            } else {
                                self.get_value_or_top(input1)
                            };
                            val0.and(&val1)
                        }

                        PcodeOperation::BoolOr { input0, input1, .. } => {
                            let val0 = if input0.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input0.offset)
                            } else {
                                self.get_value_or_top(input0)
                            };
                            let val1 = if input1.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input1.offset)
                            } else {
                                self.get_value_or_top(input1)
                            };
                            val0.or(&val1)
                        }

                        PcodeOperation::BoolXor { input0, input1, .. } => {
                            let val0 = if input0.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input0.offset)
                            } else {
                                self.get_value_or_top(input0)
                            };
                            let val1 = if input1.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input1.offset)
                            } else {
                                self.get_value_or_top(input1)
                            };
                            val0.xor(&val1)
                        }

                        PcodeOperation::BoolNegate { input, .. } => {
                            if let Some(c) = self.extract_const(input) {
                                VarnodeValue::Const(if c == 0 { 1 } else { 0 })
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        // Piece/SubPiece
                        PcodeOperation::Piece { input0, input1, .. } => {
                            if let (Some(a), Some(b)) = (self.extract_const(input0), self.extract_const(input1)) {
                                // High bits from input0, low bits from input1
                                // This is simplified - proper implementation depends on sizes
                                VarnodeValue::Const((a << (input1.size * 8)) | b)
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::SubPiece { input0, input1, .. } => {
                            if let Some(offset_const) = self.extract_const(input1) {
                                if let Some(val) = self.extract_const(input0) {
                                    VarnodeValue::Const(val >> (offset_const * 8))
                                } else {
                                    VarnodeValue::Top
                                }
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        // Load - we don't track memory, so Top
                        PcodeOperation::Load { .. } => VarnodeValue::Top,

                        // Cast - preserve value
                        PcodeOperation::Cast { input, .. } => {
                            if input.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input.offset)
                            } else {
                                self.get_value_or_top(input)
                            }
                        }

                        // PtrAdd - special handling for pointer arithmetic
                        PcodeOperation::PtrAdd { input0, input1, input2, .. } => {
                            let val0 = if input0.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input0.offset)
                            } else {
                                self.get_value_or_top(input0)
                            };

                            // input2 is the element size (must be constant)
                            let elem_size = input2.offset as i64;

                            if let Some(index) = self.extract_const(input1) {
                                let offset = (index as i64).wrapping_mul(elem_size);
                                val0.add(offset)
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::PtrSub { input0, input1, .. } => {
                            if let (Some(a), Some(b)) = (self.extract_const(input0), self.extract_const(input1)) {
                                VarnodeValue::Const(a.wrapping_sub(b))
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        // PopCount, LzCount - only track constants
                        PcodeOperation::PopCount { input, .. } => {
                            if let Some(c) = self.extract_const(input) {
                                VarnodeValue::Const(c.count_ones() as u64)
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::LzCount { input, .. } => {
                            if let Some(c) = self.extract_const(input) {
                                VarnodeValue::Const(c.leading_zeros() as u64)
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        // Everything else: Top
                        _ => VarnodeValue::Top,
                    };

                    new_state.written_locations.insert(output_vn, result_value);
                }
                GeneralizedVarNode::Indirect(_) => {
                    // Indirect writes are not tracked
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
        let next_state = self.transfer_impl(opcode.borrow());
        std::iter::once(next_state).into()
    }
}

// Strengthen implementations for compound analysis
impl crate::analysis::compound::Strengthen<crate::analysis::cpa::lattice::pcode::PcodeAddressLattice> for DirectValuationState {
    fn strengthen(&mut self, _original: &Self, location: &crate::analysis::cpa::lattice::pcode::PcodeAddressLattice, op: &jingle_sleigh::PcodeOperation) -> crate::analysis::compound::StrengthenOutcome {
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


/// The Direct Valuation CPA
///
/// This analysis can optionally track a specific varnode as an "entry" value (e.g., stack pointer).
/// If provided, this varnode will be initialized with Entry(varnode) instead of Top.
pub struct DirectValuationAnalysis {
    /// Optional varnode to track as an entry value (e.g., stack pointer)
    entry_varnode: Option<VarNode>,
}

impl DirectValuationAnalysis {
    /// Create a new DirectValuationAnalysis without any entry varnode
    pub fn new() -> Self {
        Self {
            entry_varnode: None,
        }
    }

    /// Create a new DirectValuationAnalysis with a specific entry varnode
    /// (e.g., stack pointer that starts at Entry value)
    pub fn with_entry_varnode(entry_varnode: VarNode) -> Self {
        Self {
            entry_varnode: Some(entry_varnode),
        }
    }
}

impl Default for DirectValuationAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurableProgramAnalysis for DirectValuationAnalysis {
    type State = DirectValuationState;
}

impl Analysis for DirectValuationAnalysis {
    type Input = DirectValuationState;

    fn make_initial_state(&self, _addr: ConcretePcodeAddress) -> Self::Input {
        let mut state = DirectValuationState::new();

        // If we have an entry varnode, initialize it
        if let Some(ref entry_vn) = self.entry_varnode {
            state.written_locations.insert(
                entry_vn.clone(),
                VarnodeValue::Entry(entry_vn.clone()),
            );
        }

        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varnode_value_ordering() {
        let bottom = VarnodeValue::Bottom;
        let const_10 = VarnodeValue::Const(10);
        let const_20 = VarnodeValue::Const(20);
        let top = VarnodeValue::Top;

        assert!(bottom < const_10);
        assert!(bottom < top);
        assert!(const_10 < top);
        assert!(const_10.partial_cmp(&const_20).is_none());
    }

    #[test]
    fn test_varnode_value_join() {
        let mut val1 = VarnodeValue::Const(10);
        let val2 = VarnodeValue::Const(10);
        val1.join(&val2);
        assert_eq!(val1, VarnodeValue::Const(10));

        let mut val1 = VarnodeValue::Const(10);
        let val2 = VarnodeValue::Const(20);
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

        let new_state = state.transfer_impl(&op);
        assert_eq!(
            new_state.get_value(&output),
            Some(&VarnodeValue::Const(42))
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

        let new_state = state.transfer_impl(&op);
        assert_eq!(new_state.get_value(&output), Some(&VarnodeValue::Top));
    }
}

