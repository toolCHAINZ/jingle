use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::residue::EmptyResidue;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::varnode_map::VarNodeMap;
use crate::display::JingleDisplay;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use internment::Intern;
use jingle_sleigh::{GeneralizedVarNode, PcodeOperation, SleighArchInfo, SpaceType, VarNode};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};

/// Symbolic valuation built from varnodes and constants.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SimpleValuation {
    Entry(Intern<VarNode>),
    Const(Intern<VarNode>),

    // Binary operators now use interned children (via `internment`) rather than Arc'd tuples.
    Mul(Intern<SimpleValuation>, Intern<SimpleValuation>),
    Add(Intern<SimpleValuation>, Intern<SimpleValuation>),
    Sub(Intern<SimpleValuation>, Intern<SimpleValuation>),
    BitAnd(Intern<SimpleValuation>, Intern<SimpleValuation>),
    BitOr(Intern<SimpleValuation>, Intern<SimpleValuation>),
    BitXor(Intern<SimpleValuation>, Intern<SimpleValuation>),
    Or(Intern<SimpleValuation>, Intern<SimpleValuation>),

    // Unary operators remain single interned child
    BitNegate(Intern<SimpleValuation>),
    Load(Intern<SimpleValuation>),
    Top,
}

impl SimpleValuation {
    fn from_varnode_or_entry(state: &SimpleValuationState, vn: &VarNode) -> Self {
        if vn.space_index == VarNode::CONST_SPACE_INDEX {
            SimpleValuation::Const(Intern::new(vn.clone()))
        } else if let Some(v) = state.written_locations.get(vn) {
            v.clone()
        } else {
            SimpleValuation::Entry(Intern::new(vn.clone()))
        }
    }

    /// Extract constant value if this is a Const variant
    pub fn as_const(&self) -> Option<u64> {
        match self {
            SimpleValuation::Const(vn) => Some(vn.as_ref().offset),
            _ => None,
        }
    }

    /// Returns true if this valuation's root node is a unit variant
    /// or has only a single child
    pub fn is_unit_expression(&self) -> bool {
        match self {
            SimpleValuation::Entry(_)
            | SimpleValuation::Const(_)
            | SimpleValuation::BitNegate(_)
            | SimpleValuation::Load(_)
            | SimpleValuation::Top => true,
            _ => false,
        }
    }

    /// Create a constant VarNode with the given value and size
    fn make_const(value: u64, size: usize) -> Self {
        SimpleValuation::Const(Intern::new(VarNode {
            space_index: VarNode::CONST_SPACE_INDEX,
            offset: value,
            size,
        }))
    }

    /// Helper to pick a reasonable size for a new constant when folding results.
    /// Prefer sizes found on Entry/Const varnodes; fall back to 8 bytes (64-bit).
    fn derive_size_from(val: &SimpleValuation) -> usize {
        match val {
            SimpleValuation::Const(vn) | SimpleValuation::Entry(vn) => vn.as_ref().size,
            _ => 8,
        }
    }

    /// Normalize commutative operands so that constants (if present) are on the right.
    /// Returns (left, right) possibly swapped.
    fn normalize_commutative(
        left: SimpleValuation,
        right: SimpleValuation,
    ) -> (SimpleValuation, SimpleValuation) {
        let left_is_const = left.as_const().is_some();
        let right_is_const = right.as_const().is_some();

        // If left is const and right is not, swap them so constant is on right.
        if left_is_const && !right_is_const {
            (right, left)
        } else {
            (left, right)
        }
    }

    /// Perform simple simplifications on the top two levels of the expression tree.
    /// This reduces expression height by folding constants and flattening nested operations.
    ///
    /// NOTE: This is now functional and returns a new simplified VarNodeValuation instead
    /// of mutating the receiver.
    fn simplify(&self) -> Self {
        match self {
            SimpleValuation::Add(a_intern, b_intern) => {
                // simplify children first
                let a_s = a_intern.as_ref().simplify();
                let b_s = b_intern.as_ref().simplify();

                // if any child is Top, the result is Top
                if matches!(a_s, SimpleValuation::Top) || matches!(b_s, SimpleValuation::Top) {
                    return SimpleValuation::Top;
                }

                // both const -> fold
                if let (Self::Const(a_vn), Self::Const(b_vn)) = (&a_s, &b_s) {
                    let mut vn = a_vn.as_ref().clone();
                    vn.offset = vn.offset.wrapping_add(b_vn.as_ref().offset);
                    return Self::Const(Intern::new(vn));
                }

                // normalization: ensure constants are on the right
                let (left, right) = Self::normalize_commutative(a_s, b_s);

                // expr + 0 -> expr
                if right.as_const() == Some(0) {
                    return left;
                }

                // ((expr + #a) + #b) -> (expr + #(a + b))
                if let SimpleValuation::Add(inner_a, inner_b) = &left {
                    if let SimpleValuation::Const(inner_const_vn) = inner_b.as_ref() {
                        if let SimpleValuation::Const(b_vn) = &right {
                            let mut vn = inner_const_vn.as_ref().clone();
                            vn.offset = vn.offset.wrapping_add(b_vn.as_ref().offset);
                            let new_const = Self::Const(Intern::new(vn));
                            return Self::Add(inner_a.clone(), Intern::new(new_const));
                        }
                    }
                }

                // default: rebuild with simplified children
                Self::Add(Intern::new(left), Intern::new(right))
            }

            SimpleValuation::Sub(a_intern, b_intern) => {
                let a_s = a_intern.as_ref().simplify();
                let b_s = b_intern.as_ref().simplify();

                if matches!(a_s, SimpleValuation::Top) || matches!(b_s, SimpleValuation::Top) {
                    return SimpleValuation::Top;
                }

                // both const -> fold
                if let (Self::Const(a_vn), Self::Const(b_vn)) = (&a_s, &b_s) {
                    let mut vn = a_vn.as_ref().clone();
                    vn.offset = vn.offset.wrapping_sub(b_vn.as_ref().offset);
                    return Self::Const(Intern::new(vn));
                }

                // expr - 0 -> expr
                if b_s.as_const() == Some(0) {
                    return a_s;
                }

                // x - x -> 0
                if a_s == b_s {
                    let size = Self::derive_size_from(&a_s);
                    return Self::make_const(0, size);
                }

                // ((expr - #a) - #b) -> (expr - #(a + b))
                if let SimpleValuation::Sub(inner_a, inner_b) = &a_s {
                    if let SimpleValuation::Const(inner_const_vn) = inner_b.as_ref() {
                        if let SimpleValuation::Const(b_vn) = &b_s {
                            let mut vn = inner_const_vn.as_ref().clone();
                            vn.offset = vn.offset.wrapping_add(b_vn.as_ref().offset);
                            let new_const = Self::Const(Intern::new(vn));
                            return Self::Sub(inner_a.clone(), Intern::new(new_const));
                        }
                    }
                }

                // todo: ((expr + #a) - #b) -> (expr + #(a - b)) if |a|>|b| or (expr - #(b - a)) if |b|>|a|

                Self::Sub(Intern::new(a_s), Intern::new(b_s))
            }

            SimpleValuation::Mul(a_intern, b_intern) => {
                let a_s = a_intern.as_ref().simplify();
                let b_s = b_intern.as_ref().simplify();

                if matches!(a_s, SimpleValuation::Top) || matches!(b_s, SimpleValuation::Top) {
                    return SimpleValuation::Top;
                }

                // normalization: prefer constant on the right
                let (left, right) = Self::normalize_commutative(a_s, b_s);

                // both const -> fold
                if let (Self::Const(a_vn), Self::Const(b_vn)) = (&left, &right) {
                    let mut vn = a_vn.as_ref().clone();
                    vn.offset = vn.offset.wrapping_mul(b_vn.as_ref().offset);
                    return Self::Const(Intern::new(vn));
                }

                // expr * 1 -> expr
                if right.as_const() == Some(1) {
                    return left;
                }

                // expr * 0 -> 0
                if right.as_const() == Some(0) {
                    let size = Self::derive_size_from(&left);
                    return Self::make_const(0, size);
                }

                Self::Mul(Intern::new(left), Intern::new(right))
            }

            SimpleValuation::BitAnd(a_intern, b_intern) => {
                let a_s = a_intern.as_ref().simplify();
                let b_s = b_intern.as_ref().simplify();

                if matches!(a_s, SimpleValuation::Top) || matches!(b_s, SimpleValuation::Top) {
                    return SimpleValuation::Top;
                }

                // normalization: constant on right
                let (left, right) = Self::normalize_commutative(a_s, b_s);

                // both const -> fold
                if let (Self::Const(a_vn), Self::Const(b_vn)) = (&left, &right) {
                    let mut vn = a_vn.as_ref().clone();
                    vn.offset = a_vn.as_ref().offset & b_vn.as_ref().offset;
                    return Self::Const(Intern::new(vn));
                }

                // x & 0 -> 0
                if right.as_const() == Some(0) {
                    let size = Self::derive_size_from(&left);
                    return Self::make_const(0, size);
                }

                // x & x -> x
                if left == right {
                    return left;
                }

                Self::BitAnd(Intern::new(left), Intern::new(right))
            }

            SimpleValuation::BitOr(a_intern, b_intern) => {
                let a_s = a_intern.as_ref().simplify();
                let b_s = b_intern.as_ref().simplify();

                if matches!(a_s, SimpleValuation::Top) || matches!(b_s, SimpleValuation::Top) {
                    return SimpleValuation::Top;
                }

                // normalization: constant on right
                let (left, right) = Self::normalize_commutative(a_s, b_s);

                // both const -> fold
                if let (Self::Const(a_vn), Self::Const(b_vn)) = (&left, &right) {
                    let mut vn = a_vn.as_ref().clone();
                    vn.offset = a_vn.as_ref().offset | b_vn.as_ref().offset;
                    return Self::Const(Intern::new(vn));
                }

                // x | 0 -> x
                if right.as_const() == Some(0) {
                    return left;
                }

                // x | x -> x
                if left == right {
                    return left;
                }

                Self::BitOr(Intern::new(left), Intern::new(right))
            }

            SimpleValuation::BitXor(a_intern, b_intern) => {
                let a_s = a_intern.as_ref().simplify();
                let b_s = b_intern.as_ref().simplify();

                if matches!(a_s, SimpleValuation::Top) || matches!(b_s, SimpleValuation::Top) {
                    return SimpleValuation::Top;
                }

                // normalization: constant on right
                let (left, right) = Self::normalize_commutative(a_s, b_s);

                // both const -> fold
                if let (Self::Const(a_vn), Self::Const(b_vn)) = (&left, &right) {
                    let mut vn = a_vn.as_ref().clone();
                    vn.offset = a_vn.as_ref().offset ^ b_vn.as_ref().offset;
                    return Self::Const(Intern::new(vn));
                }

                // x ^ 0 -> x
                if right.as_const() == Some(0) {
                    return left;
                }

                // x ^ x -> 0
                if left == right {
                    let size = Self::derive_size_from(&left);
                    return Self::make_const(0, size);
                }

                Self::BitXor(Intern::new(left), Intern::new(right))
            }

            SimpleValuation::Or(a_intern, b_intern) => {
                let a_s = a_intern.as_ref().simplify();
                let b_s = b_intern.as_ref().simplify();

                if matches!(a_s, SimpleValuation::Top) || matches!(b_s, SimpleValuation::Top) {
                    return SimpleValuation::Top;
                }

                // both const -> numeric OR (conservative fold)
                if let (Self::Const(a_vn), Self::Const(b_vn)) = (&a_s, &b_s) {
                    let mut vn = a_vn.as_ref().clone();
                    vn.offset = a_vn.as_ref().offset | b_vn.as_ref().offset;
                    return Self::Const(Intern::new(vn));
                }

                if a_s == b_s {
                    return a_s;
                }

                Self::Or(Intern::new(a_s), Intern::new(b_s))
            }

            SimpleValuation::BitNegate(a_intern) => {
                let a_s = a_intern.as_ref().simplify();

                if matches!(a_s, SimpleValuation::Top) {
                    return SimpleValuation::Top;
                }

                if let Self::Const(vn) = &a_s {
                    let mut new_vn = vn.as_ref().clone();
                    let bits = (new_vn.size as u64).saturating_mul(8);
                    let mask = if bits == 0 {
                        0u64
                    } else if bits >= 64 {
                        u64::MAX
                    } else {
                        (1u64 << (bits as u32)) - 1
                    };
                    new_vn.offset = (!new_vn.offset) & mask;
                    return Self::Const(Intern::new(new_vn));
                }

                Self::BitNegate(Intern::new(a_s))
            }

            SimpleValuation::Load(a_intern) => {
                let a_s = a_intern.as_ref().simplify();

                if matches!(a_s, SimpleValuation::Top) {
                    return SimpleValuation::Top;
                }

                Self::Load(Intern::new(a_s))
            }

            // Entry, Const, Top - nothing to simplify beyond cloning
            SimpleValuation::Entry(_) | SimpleValuation::Const(_) | SimpleValuation::Top => {
                self.clone()
            }
        }
    }
}

impl JingleDisplay for SimpleValuation {
    // todo: only wrap in parens if it's a non-unit inner expresison
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        match self {
            SimpleValuation::Entry(vn) => write!(f, "{}", vn.as_ref().display(info)),
            SimpleValuation::Const(vn) => write!(f, "{}", vn.as_ref().display(info)),
            SimpleValuation::Mul(a, b) => {
                write!(
                    f,
                    "({}*{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValuation::Add(a, b) => {
                write!(
                    f,
                    "({}+{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValuation::Sub(a, b) => {
                write!(
                    f,
                    "({}-{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValuation::BitAnd(a, b) => {
                write!(
                    f,
                    "({}&{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValuation::BitOr(a, b) => {
                write!(
                    f,
                    "({}|{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValuation::BitXor(a, b) => {
                write!(
                    f,
                    "({}^{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValuation::BitNegate(a) => write!(f, "(~{})", a.as_ref().display(info)),
            SimpleValuation::Or(a, b) => {
                write!(
                    f,
                    "({}||{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValuation::Load(a) => write!(f, "Load({})", a.as_ref().display(info)),
            SimpleValuation::Top => write!(f, "⊤"),
        }
    }
}

/// How to merge conflicting valuations for a single varnode when joining states.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MergeBehavior {
    /// Combine differing valuations into an `Or(...)` expression (higher precision).
    Or,
    /// Converge differing valuations to `Top` (lower precision, useful when locations are not unwound).
    Top,
}

/// State for the VarNodeValuation-based direct valuation CPA.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SimpleValuationState {
    written_locations: VarNodeMap<SimpleValuation>,
    arch_info: SleighArchInfo,
    /// Merge behavior controlling how conflicting valuations are handled during `join`.
    merge_behavior: MergeBehavior,
}

impl Hash for SimpleValuationState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // `VarNodeMap` stores keys in sorted order; iterate deterministically.
        for (vn, val) in self.written_locations.iter() {
            vn.hash(state);
            val.hash(state);
        }
        // include merge behavior in the hash so states with different merge behaviors are distinct
        self.merge_behavior.hash(state);
        self.arch_info.hash(state);
    }
}

impl Display for SimpleValuationState {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        let hash_value = hasher.finish();
        write!(f, "Hash({:016x})", hash_value)
    }
}

impl JingleDisplay for SimpleValuationState {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        // Render the written locations in a concise form using the Sleigh arch display context.
        write!(f, "SimpleValuationState {{")?;
        let mut first = true;
        for (vn, val) in self.written_locations.iter() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            // Use the JingleDisplay implementations for VarNode and SimpleValuation
            write!(f, "{} = {}", vn.display(info), val.display(info))?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

impl SimpleValuationState {
    /// Create a new state with the default merge behavior of `Or`.
    pub fn new(arch_info: SleighArchInfo) -> Self {
        Self {
            written_locations: VarNodeMap::new(),
            arch_info,
            merge_behavior: MergeBehavior::Or,
        }
    }

    /// Create a new state specifying the desired merge behavior.
    pub fn new_with_behavior(arch_info: SleighArchInfo, merge_behavior: MergeBehavior) -> Self {
        Self {
            written_locations: VarNodeMap::new(),
            arch_info,
            merge_behavior,
        }
    }

    pub fn get_value(&self, varnode: &VarNode) -> Option<&SimpleValuation> {
        self.written_locations.get(varnode)
    }

    pub fn written_locations(&self) -> &VarNodeMap<SimpleValuation> {
        &self.written_locations
    }

    /// Transfer function: build symbolic valuations for pcode operations.
    ///
    /// Note: This returns a new state (functional) instead of mutating in place.
    fn transfer_impl(&self, op: &PcodeOperation) -> Self {
        let mut new_state = self.clone();

        if let Some(output) = op.output() {
            match output {
                GeneralizedVarNode::Direct(output_vn) => {
                    let result_val = match op {
                        // Copy
                        PcodeOperation::Copy { input, .. } => {
                            if input.space_index == VarNode::CONST_SPACE_INDEX {
                                SimpleValuation::Const(Intern::new(input.clone()))
                            } else {
                                SimpleValuation::from_varnode_or_entry(self, input)
                            }
                        }

                        PcodeOperation::IntAdd { input0, input1, .. } => {
                            let a = SimpleValuation::from_varnode_or_entry(self, input0);
                            let b = SimpleValuation::from_varnode_or_entry(self, input1);
                            SimpleValuation::Add(Intern::new(a), Intern::new(b))
                        }

                        PcodeOperation::IntSub { input0, input1, .. } => {
                            let a = SimpleValuation::from_varnode_or_entry(self, input0);
                            let b = SimpleValuation::from_varnode_or_entry(self, input1);
                            SimpleValuation::Sub(Intern::new(a), Intern::new(b))
                        }

                        PcodeOperation::IntMult { input0, input1, .. } => {
                            let a = SimpleValuation::from_varnode_or_entry(self, input0);
                            let b = SimpleValuation::from_varnode_or_entry(self, input1);
                            SimpleValuation::Mul(Intern::new(a), Intern::new(b))
                        }

                        // Bitwise operations
                        PcodeOperation::IntAnd { input0, input1, .. }
                        | PcodeOperation::BoolAnd { input0, input1, .. } => {
                            let a = SimpleValuation::from_varnode_or_entry(self, input0);
                            let b = SimpleValuation::from_varnode_or_entry(self, input1);
                            SimpleValuation::BitAnd(Intern::new(a), Intern::new(b))
                        }

                        PcodeOperation::IntXor { input0, input1, .. }
                        | PcodeOperation::BoolXor { input0, input1, .. } => {
                            let a = SimpleValuation::from_varnode_or_entry(self, input0);
                            let b = SimpleValuation::from_varnode_or_entry(self, input1);
                            SimpleValuation::BitXor(Intern::new(a), Intern::new(b))
                        }

                        PcodeOperation::IntOr { input0, input1, .. }
                        | PcodeOperation::BoolOr { input0, input1, .. } => {
                            let a = SimpleValuation::from_varnode_or_entry(self, input0);
                            let b = SimpleValuation::from_varnode_or_entry(self, input1);
                            SimpleValuation::BitOr(Intern::new(a), Intern::new(b))
                        }
                        PcodeOperation::IntLeftShift { input0, input1, .. }
                        | PcodeOperation::IntRightShift { input0, input1, .. }
                        | PcodeOperation::IntSignedRightShift { input0, input1, .. } => {
                            // Approximate shifts as an Add of the operands (conservative symbolic form)
                            let a = SimpleValuation::from_varnode_or_entry(self, input0);
                            let b = SimpleValuation::from_varnode_or_entry(self, input1);
                            SimpleValuation::Add(Intern::new(a), Intern::new(b))
                        }

                        PcodeOperation::IntNegate { input, .. } => {
                            // Represent negate as Sub(Const(0), input) using make_const
                            let a = SimpleValuation::make_const(0, input.size);
                            let b = SimpleValuation::from_varnode_or_entry(self, input);
                            SimpleValuation::Sub(Intern::new(a), Intern::new(b))
                        }

                        PcodeOperation::Int2Comp { input, .. } => {
                            // Approximate two's complement by bit-negation
                            let a = SimpleValuation::from_varnode_or_entry(self, input);
                            SimpleValuation::BitNegate(Intern::new(a))
                        }

                        // Load - track pointer expression
                        PcodeOperation::Load { input, .. } => {
                            let ptr = &input.pointer_location;
                            let pv = if ptr.space_index == VarNode::CONST_SPACE_INDEX {
                                SimpleValuation::Const(Intern::new(ptr.clone()))
                            } else {
                                SimpleValuation::from_varnode_or_entry(self, ptr)
                            };
                            SimpleValuation::Load(Intern::new(pv))
                        }

                        // Casts/extensions - preserve symbolic value
                        PcodeOperation::IntSExt { input, .. }
                        | PcodeOperation::IntZExt { input, .. } => {
                            SimpleValuation::from_varnode_or_entry(self, input)
                        }

                        // Default: be conservative and mark as Top
                        _ => SimpleValuation::Top,
                    };
                    // simplify returns a new value
                    let simplified = result_val.simplify();
                    new_state.written_locations.insert(output_vn, simplified);
                }

                GeneralizedVarNode::Indirect(_) => {
                    // Indirect writes are not tracked by this CPA.
                }
            }
        }

        // Clear internal-space varnodes on control-flow to non-const destinations (same policy as direct_valuation.rs)
        match op {
            PcodeOperation::Branch { input } | PcodeOperation::CBranch { input0: input, .. } => {
                if input.space_index != VarNode::CONST_SPACE_INDEX {
                    // VarNodeMap doesn't provide `retain`; collect keys to remove and remove them.
                    let mut to_remove: Vec<VarNode> = Vec::new();
                    for (vn, _) in new_state.written_locations.iter() {
                        let keep = self
                            .arch_info
                            .get_space(vn.space_index)
                            .map(|space| space._type != SpaceType::IPTR_INTERNAL)
                            .unwrap_or(true);
                        if !keep {
                            to_remove.push(vn.clone());
                        }
                    }
                    for k in to_remove {
                        new_state.written_locations.remove(&k);
                    }
                }
            }
            PcodeOperation::BranchInd { .. } | PcodeOperation::CallInd { .. } => {
                // Similar retain behavior as above for branch-indirect.
                let mut to_remove: Vec<VarNode> = Vec::new();
                for (vn, _) in new_state.written_locations.iter() {
                    let keep = self
                        .arch_info
                        .get_space(vn.space_index)
                        .map(|space| space._type != SpaceType::IPTR_INTERNAL)
                        .unwrap_or(true);
                    if !keep {
                        to_remove.push(vn.clone());
                    }
                }
                for k in to_remove {
                    new_state.written_locations.remove(&k);
                }
            }
            _ => {}
        }

        new_state
    }
}

impl PartialOrd for SimpleValuation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

impl JoinSemiLattice for SimpleValuation {
    fn join(&mut self, _other: &Self) {}
}

impl PartialOrd for SimpleValuationState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Make states comparable only when they have the same keys and identical valuations.
        if self.written_locations.len() != other.written_locations.len() {
            return None;
        }

        for (key, val) in self.written_locations.iter() {
            match other.written_locations.get(key) {
                Some(other_val) => {
                    if val != other_val {
                        return None;
                    }
                }
                None => return None,
            }
        }

        Some(Ordering::Equal)
    }
}

impl JoinSemiLattice for SimpleValuationState {
    fn join(&mut self, other: &Self) {
        // For each varnode present in `other`:
        // - if present in self with same valuation -> keep
        // - if present in self with different valuation -> combine according to merge_behavior
        // - if absent in self -> clone from other
        for (key, other_val) in other.written_locations.iter() {
            match self.written_locations.get_mut(key) {
                Some(my_val) => {
                    if my_val == &SimpleValuation::Top || other_val == &SimpleValuation::Top {
                        *my_val = SimpleValuation::Top;
                    } else if my_val != other_val {
                        match self.merge_behavior {
                            MergeBehavior::Or => {
                                // create Or(...) of the two, then simplify the result
                                let combined = SimpleValuation::Or(
                                    Intern::new(my_val.clone()),
                                    Intern::new(other_val.clone()),
                                );
                                *my_val = combined.simplify();
                            }
                            MergeBehavior::Top => {
                                // converge differing values to Top (less precise, but useful when not unwinding locations)
                                *my_val = SimpleValuation::Top;
                            }
                        }
                    }
                }
                None => {
                    self.written_locations
                        .insert(key.clone(), other_val.clone());
                }
            }
        }
    }
}

impl AbstractState for SimpleValuationState {
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

pub struct SimpleValuationAnalysis {
    arch_info: SleighArchInfo,
    /// Default merge behavior for states produced by this analysis.
    merge_behavior: MergeBehavior,
}

impl SimpleValuationAnalysis {
    /// Create with the default merge behavior (`Or`).
    pub fn new(arch_info: SleighArchInfo, merge_behavior: MergeBehavior) -> Self {
        Self {
            arch_info,
            merge_behavior,
        }
    }
}

impl ConfigurableProgramAnalysis for SimpleValuationAnalysis {
    type State = SimpleValuationState;
    type Reducer<'op> = EmptyResidue<Self::State>;
}

impl IntoState<SimpleValuationAnalysis> for ConcretePcodeAddress {
    fn into_state(
        self,
        c: &SimpleValuationAnalysis,
    ) -> <SimpleValuationAnalysis as ConfigurableProgramAnalysis>::State {
        SimpleValuationState {
            written_locations: VarNodeMap::new(),
            arch_info: c.arch_info.clone(),
            merge_behavior: c.merge_behavior,
        }
    }
}
