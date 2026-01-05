//! Stack Offset Analysis
//!
//! This module provides a Configurable Program Analysis for tracking the relative offset
//! of the stack pointer throughout program execution. The analysis tracks how the stack
//! pointer changes relative to its initial value, which is useful for understanding
//! stack frame layout and detecting stack-related issues.
//!
//! This CPA is designed to be used in a compound analysis with location tracking,
//! so it does not track program locations itself.

use crate::analysis::compound::{Strengthen, StrengthenOutcome};
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::analysis::unwinding::UnwindingCpaState;
use jingle_sleigh::{GeneralizedVarNode, PcodeOperation, VarNode};
use std::borrow::Borrow;
use std::cmp::Ordering;

/// Represents a range of possible stack offset values
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StackOffsetLattice {
    /// No information (bottom of lattice)
    Bottom,
    /// A concrete offset value
    Offset(i64),
    /// A range of possible offsets [min, max]
    Range(i64, i64),
    /// Unknown offset (top of lattice)
    Top,
}

impl StackOffsetLattice {
    /// Create a new concrete offset
    pub fn new(offset: i64) -> Self {
        Self::Offset(offset)
    }

    /// Add a constant to the offset
    fn add(&self, delta: i64) -> Self {
        match self {
            Self::Bottom => Self::Bottom,
            Self::Offset(v) => Self::Offset(v.wrapping_add(delta)),
            Self::Range(min, max) => Self::Range(min.wrapping_add(delta), max.wrapping_add(delta)),
            Self::Top => Self::Top,
        }
    }

    /// Subtract a constant from the offset
    fn sub(&self, delta: i64) -> Self {
        match self {
            Self::Bottom => Self::Bottom,
            Self::Offset(v) => Self::Offset(v.wrapping_sub(delta)),
            Self::Range(min, max) => Self::Range(min.wrapping_sub(delta), max.wrapping_sub(delta)),
            Self::Top => Self::Top,
        }
    }

    /// Check if this is a concrete value
    pub fn is_concrete(&self) -> bool {
        matches!(self, Self::Offset(_))
    }

    /// Get the concrete value if it exists
    pub fn concrete_value(&self) -> Option<i64> {
        match self {
            Self::Offset(v) => Some(*v),
            _ => None,
        }
    }
}

impl PartialOrd for StackOffsetLattice {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use StackOffsetLattice::*;
        match (self, other) {
            // Bottom is less than everything except itself
            (Bottom, Bottom) => Some(Ordering::Equal),
            (Bottom, _) => Some(Ordering::Less),
            (_, Bottom) => Some(Ordering::Greater),

            // Top is greater than everything except itself
            (Top, Top) => Some(Ordering::Equal),
            (Top, _) => Some(Ordering::Greater),
            (_, Top) => Some(Ordering::Less),

            // Equal concrete offsets
            (Offset(a), Offset(b)) if a == b => Some(Ordering::Equal),

            // Range comparisons
            (Range(min1, max1), Range(min2, max2)) => {
                if min1 == min2 && max1 == max2 {
                    Some(Ordering::Equal)
                } else if min1 >= min2 && max1 <= max2 {
                    Some(Ordering::Less)
                } else if min2 >= min1 && max2 <= max1 {
                    Some(Ordering::Greater)
                } else {
                    None
                }
            }

            // Offset is contained in range
            (Offset(v), Range(min, max)) => {
                if v >= min && v <= max {
                    Some(Ordering::Less)
                } else {
                    None
                }
            }
            (Range(min, max), Offset(v)) => {
                if v >= min && v <= max {
                    Some(Ordering::Greater)
                } else {
                    None
                }
            }

            // Incomparable concrete offsets
            _ => None,
        }
    }
}

impl JoinSemiLattice for StackOffsetLattice {
    fn join(&mut self, other: &Self) {
        use StackOffsetLattice::*;
        *self = match (&*self, other) {
            (Bottom, x) | (x, Bottom) => x.clone(),
            (Top, _) | (_, Top) => Top,
            (Offset(a), Offset(b)) if a == b => Offset(*a),
            (Offset(a), Offset(b)) => Range((*a).min(*b), (*a).max(*b)),
            (Offset(v), Range(min, max)) | (Range(min, max), Offset(v)) => {
                Range((*v).min(*min), (*v).max(*max))
            }
            (Range(min1, max1), Range(min2, max2)) => Range((*min1).min(*min2), (*max1).max(*max2)),
        };
    }
}

/// Abstract state for stack offset analysis
///
/// This state tracks the offset of the stack pointer relative to its initial value.
/// It can be used in a product analysis with location to track stack changes at each
/// program point.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StackOffsetState {
    /// The current stack offset (relative to initial stack pointer value)
    offset: StackOffsetLattice,
    /// The varnode representing the stack pointer for this architecture
    /// This is typically RSP on x86-64, ESP on x86-32, SP on ARM, etc.
    stack_pointer: VarNode,
}

impl StackOffsetState {
    /// Create a new stack offset state with initial offset 0
    pub fn new(stack_pointer: VarNode) -> Self {
        Self {
            offset: StackOffsetLattice::Offset(0),
            stack_pointer,
        }
    }

    /// Create a new stack offset state with a specific initial offset
    pub fn with_offset(stack_pointer: VarNode, offset: i64) -> Self {
        Self {
            offset: StackOffsetLattice::Offset(offset),
            stack_pointer,
        }
    }

    /// Get the current stack offset
    pub fn offset(&self) -> &StackOffsetLattice {
        &self.offset
    }

    /// Get the stack pointer varnode
    pub fn stack_pointer(&self) -> &VarNode {
        &self.stack_pointer
    }

    /// Transfer function for stack offset analysis
    fn transfer_impl(&self, op: &PcodeOperation) -> StackOffsetLattice {
        match op {
            // Stack pointer arithmetic: SP = SP + constant
            PcodeOperation::IntAdd {
                output,
                input0,
                input1,
            } => {
                if output == &self.stack_pointer && input0 == &self.stack_pointer {
                    if let Some(delta) = Self::extract_constant(input1) {
                        return self.offset.add(delta);
                    }
                } else if output == &self.stack_pointer && input1 == &self.stack_pointer {
                    if let Some(delta) = Self::extract_constant(input0) {
                        return self.offset.add(delta);
                    }
                }
                // If SP is assigned but we can't track it precisely, go to Top
                if output == &self.stack_pointer {
                    return StackOffsetLattice::Top;
                }
                self.offset.clone()
            }

            // Stack pointer arithmetic: SP = SP - constant
            PcodeOperation::IntSub {
                output,
                input0,
                input1,
            } => {
                if output == &self.stack_pointer && input0 == &self.stack_pointer {
                    if let Some(delta) = Self::extract_constant(input1) {
                        return self.offset.sub(delta);
                    }
                }
                // If SP is assigned but we can't track it precisely, go to Top
                if output == &self.stack_pointer {
                    return StackOffsetLattice::Top;
                }
                self.offset.clone()
            }

            // Copy to/from stack pointer
            PcodeOperation::Copy { output, input } => {
                if output == &self.stack_pointer {
                    // SP is being overwritten - check if it's from a known value
                    if input == &self.stack_pointer {
                        self.offset.clone()
                    } else {
                        // SP assigned from unknown source
                        StackOffsetLattice::Top
                    }
                } else {
                    self.offset.clone()
                }
            }

            // Any other operation that writes to SP makes it unknown
            _ => {
                if Self::writes_to_varnode(op, &self.stack_pointer) {
                    StackOffsetLattice::Top
                } else {
                    self.offset.clone()
                }
            }
        }
    }

    /// Extract a constant value from a varnode if it's in the constant space
    fn extract_constant(vn: &VarNode) -> Option<i64> {
        // Constant space typically has index 0 or 1, but we check if space_index is 0
        // and interpret the offset as a signed integer
        // Note: This is a simplified implementation; real implementation should
        // check against the actual constant space index from SleighArchInfo
        if vn.space_index == VarNode::CONST_SPACE_INDEX {
            // Interpret as signed based on size
            Some(vn.offset as i64)
        } else {
            None
        }
    }

    /// Check if an operation writes to a specific varnode
    fn writes_to_varnode(op: &PcodeOperation, v: &VarNode) -> bool {
        match op.output() {
            Some(GeneralizedVarNode::Direct(vn)) => v == &vn,
            _ => false,
        }
    }
}

impl PartialOrd for StackOffsetState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // States are only comparable if they track the same stack pointer
        if self.stack_pointer != other.stack_pointer {
            None
        } else {
            self.offset.partial_cmp(&other.offset)
        }
    }
}

impl JoinSemiLattice for StackOffsetState {
    fn join(&mut self, other: &Self) {
        // Can only join if tracking the same stack pointer
        if self.stack_pointer == other.stack_pointer {
            self.offset.join(&other.offset);
        } else {
            // If tracking different stack pointers, result is Top
            self.offset = StackOffsetLattice::Top;
        }
    }
}

impl AbstractState for StackOffsetState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_join(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer<'a, B: Borrow<PcodeOperation>>(&'a self, opcode: B) -> Successor<'a, Self> {
        let new_offset = self.transfer_impl(opcode.borrow());
        let next_state = Self {
            offset: new_offset,
            stack_pointer: self.stack_pointer.clone(),
        };
        std::iter::once(next_state).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_offset_lattice_ordering() {
        let bottom = StackOffsetLattice::Bottom;
        let offset_0 = StackOffsetLattice::Offset(0);
        let offset_8 = StackOffsetLattice::Offset(8);
        let range = StackOffsetLattice::Range(-16, 16);
        let top = StackOffsetLattice::Top;

        assert!(bottom < offset_0);
        assert!(bottom < top);
        assert!(offset_0 < top);
        assert!(offset_0.partial_cmp(&offset_8).is_none());
        assert!(offset_0 < range);
        assert!(offset_8 < range);
    }

    #[test]
    fn test_stack_offset_lattice_join() {
        let mut offset_0 = StackOffsetLattice::Offset(0);
        let offset_8 = StackOffsetLattice::Offset(8);

        offset_0.join(&offset_8);
        assert_eq!(offset_0, StackOffsetLattice::Range(0, 8));

        let mut range1 = StackOffsetLattice::Range(0, 8);
        let range2 = StackOffsetLattice::Range(4, 12);
        range1.join(&range2);
        assert_eq!(range1, StackOffsetLattice::Range(0, 12));
    }

    #[test]
    fn test_stack_offset_arithmetic() {
        let offset = StackOffsetLattice::Offset(0);
        assert_eq!(offset.add(8), StackOffsetLattice::Offset(8));
        assert_eq!(offset.sub(16), StackOffsetLattice::Offset(-16));

        let range = StackOffsetLattice::Range(0, 8);
        assert_eq!(range.add(4), StackOffsetLattice::Range(4, 12));
        assert_eq!(range.sub(4), StackOffsetLattice::Range(-4, 4));
    }
}

impl Strengthen<PcodeAddressLattice> for StackOffsetState {}

impl Strengthen<UnwindingCpaState> for StackOffsetState {}
