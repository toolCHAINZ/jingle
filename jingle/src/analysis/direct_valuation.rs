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
//! let valuation_analysis = DirectValuationAnalysis::with_entry_varnode(loaded.info.clone(), stack_pointer_varnode);
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
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::residue::EmptyResidue;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, StateDisplay, Successor};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::display::JingleDisplayable;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{GeneralizedVarNode, PcodeOperation, SleighArchInfo, SpaceType, VarNode};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};

/// Represents the abstract value of a varnode in the analysis
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum VarnodeValue {
    /// The original entry value of this varnode (e.g., stack pointer at function entry)
    Entry(VarNode),
    /// A constant signed offset relative to the entry value
    Offset(VarNode, i64),
    /// A concrete constant value, not dependent on any entry value
    Const(u64),
    /// A value loaded from memory at a known pointer location
    Loaded(Box<VarnodeValue>),
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

    /// Check if this is a loaded value
    pub fn is_loaded(&self) -> bool {
        matches!(self, VarnodeValue::Loaded(_))
    }

    /// Get the loaded pointer value if it exists
    pub fn as_loaded(&self) -> Option<&VarnodeValue> {
        match self {
            VarnodeValue::Loaded(v) => Some(v.as_ref()),
            _ => None,
        }
    }

    /// Add a constant to this value
    fn add(&self, delta: i64) -> Self {
        match self {
            VarnodeValue::Entry(vn) => VarnodeValue::Offset(vn.clone(), delta),
            VarnodeValue::Offset(vn, offset) => {
                let sum = offset.wrapping_add(delta);
                if sum != 0 {
                    VarnodeValue::Offset(vn.clone(), offset.wrapping_add(delta))
                } else {
                    VarnodeValue::Entry(vn.clone())
                }
            }
            VarnodeValue::Const(val) => VarnodeValue::Const(val.wrapping_add(delta as u64)),
            VarnodeValue::Loaded(_) => VarnodeValue::Top,
            VarnodeValue::Top => VarnodeValue::Top,
        }
    }

    /// Subtract a constant from this value
    fn sub(&self, delta: i64) -> Self {
        match self {
            VarnodeValue::Entry(vn) => VarnodeValue::Offset(vn.clone(), -delta),
            VarnodeValue::Offset(vn, offset) => {
                let diff = offset.wrapping_sub(delta);
                if diff != 0 {
                    VarnodeValue::Offset(vn.clone(), diff)
                } else {
                    VarnodeValue::Entry(vn.clone())
                }
            }
            VarnodeValue::Const(val) => VarnodeValue::Const(val.wrapping_sub(delta as u64)),
            VarnodeValue::Loaded(_) => VarnodeValue::Top,
            VarnodeValue::Top => VarnodeValue::Top,
        }
    }

    /// Negate this value
    fn negate(&self) -> Self {
        match self {
            VarnodeValue::Const(val) => VarnodeValue::Const((*val as i64).wrapping_neg() as u64),
            _ => VarnodeValue::Top,
        }
    }

    /// Bitwise AND with another value
    fn and(&self, other: &Self) -> Self {
        match (self, other) {
            (VarnodeValue::Const(a), VarnodeValue::Const(b)) => VarnodeValue::Const(a & b),
            (VarnodeValue::Top, _) | (_, VarnodeValue::Top) => VarnodeValue::Top,
            _ => VarnodeValue::Top,
        }
    }

    /// Bitwise OR with another value
    fn or(&self, other: &Self) -> Self {
        match (self, other) {
            (VarnodeValue::Const(a), VarnodeValue::Const(b)) => VarnodeValue::Const(a | b),
            (VarnodeValue::Top, _) | (_, VarnodeValue::Top) => VarnodeValue::Top,
            _ => VarnodeValue::Top,
        }
    }

    /// Bitwise XOR with another value
    fn xor(&self, other: &Self) -> Self {
        match (self, other) {
            (VarnodeValue::Const(a), VarnodeValue::Const(b)) => VarnodeValue::Const(a ^ b),
            (VarnodeValue::Top, _) | (_, VarnodeValue::Top) => VarnodeValue::Top,
            _ => VarnodeValue::Top,
        }
    }
}

impl JingleDisplayable for VarnodeValue {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        match self {
            VarnodeValue::Entry(vn) => write!(f, "Entry({})", vn.display(info)),
            VarnodeValue::Offset(vn, offset) => {
                if *offset >= 0 {
                    write!(f, "{}+{:#x}", vn.display(info), offset)
                } else {
                    write!(f, "{}-{:#x}", vn.display(info), offset)
                }
            }
            VarnodeValue::Const(val) => write!(f, "{:#x}", val),
            VarnodeValue::Loaded(ptr_val) => {
                write!(f, "Load({})", ptr_val.display(info))
            }
            VarnodeValue::Top => write!(f, "⊤"),
        }
    }
}

impl PartialOrd for VarnodeValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use VarnodeValue::*;
        match (self, other) {
            // Bottom is less than everything except itself
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
            (Offset(a, off_a), Offset(b, off_b)) if a == b && off_a == off_b => {
                Some(Ordering::Equal)
            }

            // Equal constants
            (Const(a), Const(b)) if a == b => Some(Ordering::Equal),

            // Loaded values
            (Loaded(a), Loaded(b)) => a.partial_cmp(b),

            // Everything else is incomparable
            _ => None,
        }
    }
}

impl JoinSemiLattice for VarnodeValue {
    fn join(&mut self, other: &Self) {
        use VarnodeValue::*;
        *self = match (&*self, other) {
            (Top, _) | (_, Top) => Top,
            (Entry(a), Entry(b)) if a == b => Entry(a.clone()),
            (Entry(a), Offset(b, off)) | (Offset(b, off), Entry(a)) if a == b => {
                Offset(a.clone(), *off)
            }
            (Offset(a, off_a), Offset(b, off_b)) if a == b && off_a == off_b => {
                Offset(a.clone(), *off_a)
            }
            (Const(a), Const(b)) if a == b => Const(*a),
            (Loaded(a), Loaded(b)) => {
                let mut joined = a.as_ref().clone();
                joined.join(b.as_ref());
                if joined == Top {
                    Top
                } else {
                    Loaded(Box::new(joined))
                }
            }
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
    /// Architecture information for space type lookups
    arch_info: SleighArchInfo,
}

impl Hash for DirectValuationState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut sorted = self.written_locations.keys().collect::<Vec<_>>();
        sorted.sort_by_key(|k| (k.space_index, k.offset, k.size));
        for vn in sorted.iter() {
            vn.hash(state);
            self.written_locations[vn].hash(state);
        }
        self.arch_info.hash(state);
    }
}

impl StateDisplay for DirectValuationState {
    fn fmt_state(&self, f: &mut Formatter<'_>) -> FmtResult {
        // Compute hash using the same algorithm as the Hash impl
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        let hash_value = hasher.finish();
        write!(f, "Hash({:016x})", hash_value)
    }
}

impl DirectValuationState {
    /// Create a new empty direct valuation state
    pub fn new(arch_info: SleighArchInfo) -> Self {
        Self {
            written_locations: HashMap::new(),
            arch_info,
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
            .unwrap_or(VarnodeValue::Entry(varnode.clone()))
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
                                (VarnodeValue::Offset(a, off_a), VarnodeValue::Entry(b))
                                    if a == b =>
                                {
                                    VarnodeValue::Const(off_a as u64)
                                }
                                (
                                    VarnodeValue::Offset(a, off_a),
                                    VarnodeValue::Offset(b, off_b),
                                ) if a == b => VarnodeValue::Const((off_a - off_b) as u64),
                                _ => VarnodeValue::Top,
                            }
                        }

                        PcodeOperation::IntMult { input0, input1, .. } => {
                            if let (Some(a), Some(b)) =
                                (self.extract_const(input0), self.extract_const(input1))
                            {
                                VarnodeValue::Const(a.wrapping_mul(b))
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::IntDiv { input0, input1, .. } => {
                            if let (Some(a), Some(b)) =
                                (self.extract_const(input0), self.extract_const(input1))
                            {
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
                            if let (Some(a), Some(b)) =
                                (self.extract_const(input0), self.extract_const(input1))
                            {
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
                            if let (Some(a), Some(b)) =
                                (self.extract_const(input0), self.extract_const(input1))
                            {
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
                            if let (Some(a), Some(b)) =
                                (self.extract_const(input0), self.extract_const(input1))
                            {
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
                            if let (Some(a), Some(b)) =
                                (self.extract_const(input0), self.extract_const(input1))
                            {
                                VarnodeValue::Const(a.wrapping_shl(b as u32))
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::IntRightShift { input0, input1, .. } => {
                            if let (Some(a), Some(b)) =
                                (self.extract_const(input0), self.extract_const(input1))
                            {
                                VarnodeValue::Const(a.wrapping_shr(b as u32))
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        PcodeOperation::IntSignedRightShift { input0, input1, .. } => {
                            if let (Some(a), Some(b)) =
                                (self.extract_const(input0), self.extract_const(input1))
                            {
                                VarnodeValue::Const((a as i64).wrapping_shr(b as u32) as u64)
                            } else {
                                VarnodeValue::Top
                            }
                        }

                        // Sign/Zero extension - preserve constants
                        PcodeOperation::IntSExt { input, .. }
                        | PcodeOperation::IntZExt { input, .. } => {
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
                            if let (Some(a), Some(b)) =
                                (self.extract_const(input0), self.extract_const(input1))
                            {
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

                        // Load - track the pointer value if known
                        PcodeOperation::Load { input, .. } => {
                            // Get the value of the pointer
                            let pointer = &input.pointer_location;
                            let pointer_value = if pointer.space_index == VarNode::CONST_SPACE_INDEX
                            {
                                VarnodeValue::Const(pointer.offset)
                            } else {
                                self.get_value_or_top(pointer)
                            };

                            // Wrap it in Loaded to indicate this was loaded from memory
                            VarnodeValue::Loaded(Box::new(pointer_value))
                        }

                        // Cast - preserve value
                        PcodeOperation::Cast { input, .. } => {
                            if input.space_index == VarNode::CONST_SPACE_INDEX {
                                VarnodeValue::Const(input.offset)
                            } else {
                                self.get_value_or_top(input)
                            }
                        }

                        // PtrAdd - special handling for pointer arithmetic
                        PcodeOperation::PtrAdd {
                            input0,
                            input1,
                            input2,
                            ..
                        } => {
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
                            if let (Some(a), Some(b)) =
                                (self.extract_const(input0), self.extract_const(input1))
                            {
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

        // Clear internal space varnodes on control flow operations to non-const destinations
        match op {
            PcodeOperation::Branch { input } | PcodeOperation::CBranch { input0: input, .. } => {
                // Check if the branch destination is NOT in the const space
                if input.space_index != VarNode::CONST_SPACE_INDEX {
                    // Clear all varnodes in internal spaces
                    new_state.written_locations.retain(|vn, _| {
                        self.arch_info
                            .get_space(vn.space_index)
                            .map(|space| space._type != SpaceType::IPTR_INTERNAL)
                            .unwrap_or(true) // Keep if space info not found
                    });
                }
            }
            PcodeOperation::BranchInd { .. } => {
                // Indirect branches always go to non-const space, so clear internal varnodes
                new_state.written_locations.retain(|vn, _| {
                    self.arch_info
                        .get_space(vn.space_index)
                        .map(|space| space._type != SpaceType::IPTR_INTERNAL)
                        .unwrap_or(true)
                });
            }
            _ => {}
        }

        new_state
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

/// The Direct Valuation CPA
///
/// This analysis can optionally track a specific varnode as an "entry" value (e.g., stack pointer).
/// If provided, this varnode will be initialized with Entry(varnode) instead of Top.
pub struct DirectValuationAnalysis {
    /// Architecture information for space type lookups
    arch_info: SleighArchInfo,
}

impl DirectValuationAnalysis {
    /// Create a new DirectValuationAnalysis without any entry varnode
    pub fn new(arch_info: SleighArchInfo) -> Self {
        Self { arch_info }
    }

    /// Create a new DirectValuationAnalysis with a specific entry varnode
    /// (e.g., stack pointer that starts at Entry value)
    pub fn with_entry_varnode(arch_info: SleighArchInfo, _entry_varnode: VarNode) -> Self {
        Self { arch_info }
    }
}

impl ConfigurableProgramAnalysis for DirectValuationAnalysis {
    type State = DirectValuationState;
    type Reducer = EmptyResidue<Self::State>;
}

impl IntoState<DirectValuationAnalysis> for ConcretePcodeAddress {
    fn into_state(
        self,
        c: &DirectValuationAnalysis,
    ) -> <DirectValuationAnalysis as ConfigurableProgramAnalysis>::State {
        DirectValuationState {
            written_locations: Default::default(),
            arch_info: c.arch_info.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jingle_sleigh::{SleighEndianness, SpaceInfo};

    // Helper to create a mock SleighArchInfo for testing
    fn mock_arch_info() -> SleighArchInfo {
        let spaces = vec![
            SpaceInfo {
                name: "const".to_string(),
                index: 0,
                index_size_bytes: 8,
                word_size_bytes: 1,
                _type: SpaceType::IPTR_CONSTANT,
                endianness: SleighEndianness::Little,
            },
            SpaceInfo {
                name: "ram".to_string(),
                index: 1,
                index_size_bytes: 8,
                word_size_bytes: 1,
                _type: SpaceType::IPTR_PROCESSOR,
                endianness: SleighEndianness::Little,
            },
            SpaceInfo {
                name: "register".to_string(),
                index: 2,
                index_size_bytes: 8,
                word_size_bytes: 1,
                _type: SpaceType::IPTR_PROCESSOR,
                endianness: SleighEndianness::Little,
            },
            SpaceInfo {
                name: "unique".to_string(),
                index: 3,
                index_size_bytes: 8,
                word_size_bytes: 1,
                _type: SpaceType::IPTR_INTERNAL,
                endianness: SleighEndianness::Little,
            },
        ];
        SleighArchInfo::new(
            "test:LE:64:default".to_string(),
            std::iter::empty(),
            spaces.into_iter(),
            1,
            vec![],
        )
    }

    #[test]
    fn test_varnode_value_ordering() {
        let const_10 = VarnodeValue::Const(10);
        let const_20 = VarnodeValue::Const(20);
        let top = VarnodeValue::Top;

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
        let state = DirectValuationState::new(mock_arch_info());
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
        assert_eq!(new_state.get_value(&output), Some(&VarnodeValue::Const(42)));
    }

    #[test]
    fn test_copy_from_non_constant() {
        let state = DirectValuationState::new(mock_arch_info());
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
        assert_eq!(
            new_state.get_value(&output),
            Some(&VarnodeValue::Entry(input))
        );
    }

    #[test]
    fn test_branch_clears_internal_space() {
        let mut state = DirectValuationState::new(mock_arch_info());

        // Add some tracked varnodes in different spaces
        let ram_vn = VarNode {
            space_index: 1, // ram (PROCESSOR space)
            offset: 100,
            size: 8,
        };
        let reg_vn = VarNode {
            space_index: 2, // register (PROCESSOR space)
            offset: 8,
            size: 8,
        };
        let unique_vn = VarNode {
            space_index: 3, // unique (INTERNAL space)
            offset: 0x1000,
            size: 8,
        };

        state
            .written_locations
            .insert(ram_vn.clone(), VarnodeValue::Const(42));
        state
            .written_locations
            .insert(reg_vn.clone(), VarnodeValue::Const(100));
        state
            .written_locations
            .insert(unique_vn.clone(), VarnodeValue::Const(200));

        // Branch to a non-const destination (ram space)
        let branch_dest = VarNode {
            space_index: 1, // ram space
            offset: 0x1000,
            size: 8,
        };
        let branch_op = PcodeOperation::Branch { input: branch_dest };

        let new_state = state.transfer_impl(&branch_op);

        // Processor space varnodes should be retained
        assert_eq!(new_state.get_value(&ram_vn), Some(&VarnodeValue::Const(42)));
        assert_eq!(
            new_state.get_value(&reg_vn),
            Some(&VarnodeValue::Const(100))
        );

        // Internal space varnodes should be cleared
        assert_eq!(new_state.get_value(&unique_vn), None);
    }

    #[test]
    fn test_branch_to_const_does_not_clear() {
        let mut state = DirectValuationState::new(mock_arch_info());

        // Add a tracked varnode in internal space
        let unique_vn = VarNode {
            space_index: 3, // unique (INTERNAL space)
            offset: 0x1000,
            size: 8,
        };
        state
            .written_locations
            .insert(unique_vn.clone(), VarnodeValue::Const(200));

        // Branch to a const space destination (e.g., for relative branching within an instruction)
        let branch_dest = VarNode {
            space_index: VarNode::CONST_SPACE_INDEX,
            offset: 0x10,
            size: 8,
        };
        let branch_op = PcodeOperation::Branch { input: branch_dest };

        let new_state = state.transfer_impl(&branch_op);

        // Internal space varnodes should NOT be cleared for const-space branches
        assert_eq!(
            new_state.get_value(&unique_vn),
            Some(&VarnodeValue::Const(200))
        );
    }

    #[test]
    fn test_cbranch_clears_internal_space() {
        let mut state = DirectValuationState::new(mock_arch_info());

        let unique_vn = VarNode {
            space_index: 3, // unique (INTERNAL space)
            offset: 0x1000,
            size: 8,
        };
        state
            .written_locations
            .insert(unique_vn.clone(), VarnodeValue::Const(200));

        // Conditional branch to a non-const destination
        let branch_dest = VarNode {
            space_index: 1, // ram space
            offset: 0x1000,
            size: 8,
        };
        let condition = VarNode {
            space_index: 2,
            offset: 0,
            size: 1,
        };
        let cbranch_op = PcodeOperation::CBranch {
            input0: branch_dest,
            input1: condition,
        };

        let new_state = state.transfer_impl(&cbranch_op);

        // Internal space varnodes should be cleared
        assert_eq!(new_state.get_value(&unique_vn), None);
    }

    #[test]
    fn test_varnode_value_display() {
        let info = mock_arch_info();

        // Test Top
        let top = VarnodeValue::Top;
        assert_eq!(format!("{}", top.display(&info)), "⊤");

        // Test Const
        let const_val = VarnodeValue::Const(0x42);
        assert_eq!(format!("{}", const_val.display(&info)), "0x42");

        // Test Entry
        let reg_vn = VarNode {
            space_index: 2,
            offset: 8,
            size: 8,
        };
        let entry = VarnodeValue::Entry(reg_vn.clone());
        let display_str = format!("{}", entry.display(&info));
        assert!(display_str.contains("Entry"));

        // Test Offset with positive offset
        let offset_pos = VarnodeValue::Offset(reg_vn.clone(), 16);
        let display_str = format!("{}", offset_pos.display(&info));
        assert!(display_str.contains("+0x10"));

        // Test Offset with negative offset
        let offset_neg = VarnodeValue::Offset(reg_vn.clone(), -8);
        let display_str = format!("{}", offset_neg.display(&info));
        assert!(display_str.contains("-0x8") || display_str.contains("0xfffffffffffffff8"));

        // Test Loaded
        let loaded = VarnodeValue::Loaded(Box::new(VarnodeValue::Const(0x1000)));
        let display_str = format!("{}", loaded.display(&info));
        assert!(display_str.contains("Load"));
        assert!(display_str.contains("0x1000"));
    }
}
