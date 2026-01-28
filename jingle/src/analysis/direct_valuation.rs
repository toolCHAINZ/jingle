use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::residue::EmptyResidue;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::varnode_map::VarNodeMap;
use crate::display::JingleDisplayable;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{GeneralizedVarNode, PcodeOperation, SleighArchInfo, SpaceType, VarNode};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Symbolic valuation built from varnodes and constants.
///
/// This valuation intentionally does not include a Top element. Unknown or conflicting
/// information is handled at the state join level by reverting the varnode to the
/// `Entry(varnode)` form. This is acceptable for unwound / bounded analyses.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum VarNodeValuation {
    Entry(VarNode),
    Const(VarNode),

    // Binary operators now use a single Arc'ed tuple rather than two boxed children.
    Mult(Arc<(VarNodeValuation, VarNodeValuation)>),
    Add(Arc<(VarNodeValuation, VarNodeValuation)>),
    Sub(Arc<(VarNodeValuation, VarNodeValuation)>),
    BitAnd(Arc<(VarNodeValuation, VarNodeValuation)>),
    BitOr(Arc<(VarNodeValuation, VarNodeValuation)>),
    BitXor(Arc<(VarNodeValuation, VarNodeValuation)>),
    Or(Arc<(VarNodeValuation, VarNodeValuation)>),

    // Unary operators remain single Arc child
    BitNegate(Arc<VarNodeValuation>),
    Load(Arc<VarNodeValuation>),
    Top,
}

impl VarNodeValuation {
    fn from_varnode_or_entry(state: &DirectValuationState, vn: &VarNode) -> Self {
        if vn.space_index == VarNode::CONST_SPACE_INDEX {
            VarNodeValuation::Const(vn.clone())
        } else if let Some(v) = state.written_locations.get(vn) {
            v.clone()
        } else {
            VarNodeValuation::Entry(vn.clone())
        }
    }

    #[allow(dead_code)]
    fn from_varnode_or_entry_simple(vn: &VarNode) -> Self {
        if vn.space_index == VarNode::CONST_SPACE_INDEX {
            VarNodeValuation::Const(vn.clone())
        } else {
            VarNodeValuation::Entry(vn.clone())
        }
    }

    /// Extract constant value if this is a Const variant
    pub fn as_const(&self) -> Option<u64> {
        match self {
            VarNodeValuation::Const(vn) => Some(vn.offset),
            _ => None,
        }
    }

    /// Create a constant VarNode with the given value and size
    fn make_const(value: u64, size: usize) -> Self {
        VarNodeValuation::Const(VarNode {
            space_index: VarNode::CONST_SPACE_INDEX,
            offset: value,
            size,
        })
    }

    /// Perform simple simplifications on the top two levels of the expression tree.
    /// This reduces expression height by folding constants and flattening nested operations.
    ///
    /// NOTE: This is now functional and returns a new simplified VarNodeValuation instead
    /// of mutating the receiver.
    fn simplify(&self) -> Self {
        match self {
            // Arithmetic operations
            VarNodeValuation::Add(ab) => {
                let pair = ab.as_ref();
                let a = pair.0.simplify();
                let b = pair.1.simplify();

                // Const + Const = Const
                if let (Some(av), Some(bv)) = (a.as_const(), b.as_const()) {
                    if let VarNodeValuation::Const(vn) = &a {
                        let size = vn.size;
                        return Self::make_const(av.wrapping_add(bv), size);
                    }
                }

                // Flatten nested Add with Const: (Add(x, Const(c1)) + Const(c2)) -> Add(x, Const(c1+c2))
                if let Some(bv) = b.as_const() {
                    if let VarNodeValuation::Add(inner_ab) = &a {
                        let inner_pair = inner_ab.as_ref();
                        let inner_a = inner_pair.0.clone();
                        let inner_b = inner_pair.1.clone();
                        if let Some(inner_bv) = inner_b.as_const() {
                            if let VarNodeValuation::Const(vn) = &inner_b {
                                let size = vn.size;
                                let new_inner_b = Self::make_const(inner_bv.wrapping_add(bv), size);
                                return VarNodeValuation::Add(Arc::new((inner_a, new_inner_b)));
                            }
                        }
                    }
                }

                // Symmetric case: Const(c1) + Add(x, Const(c2)) -> Add(x, Const(c1+c2))
                if let Some(av) = a.as_const() {
                    if let VarNodeValuation::Add(inner_ab) = &b {
                        let inner_pair = inner_ab.as_ref();
                        let inner_a = inner_pair.0.clone();
                        let inner_b = inner_pair.1.clone();
                        if let Some(inner_bv) = inner_b.as_const() {
                            if let VarNodeValuation::Const(vn) = &inner_b {
                                let size = vn.size;
                                let new_inner_b = Self::make_const(av.wrapping_add(inner_bv), size);
                                return VarNodeValuation::Add(Arc::new((inner_a, new_inner_b)));
                            }
                        }
                    }
                }

                VarNodeValuation::Add(Arc::new((a, b)))
            }

            VarNodeValuation::Sub(ab) => {
                let pair = ab.as_ref();
                let a = pair.0.simplify();
                let b = pair.1.simplify();

                // Const - Const = Const
                if let (Some(av), Some(bv)) = (a.as_const(), b.as_const()) {
                    if let VarNodeValuation::Const(vn) = &a {
                        let size = vn.size;
                        return Self::make_const(av.wrapping_sub(bv), size);
                    }
                }

                // x - Const(0) = x
                if let Some(0) = b.as_const() {
                    return a;
                }

                VarNodeValuation::Sub(Arc::new((a, b)))
            }

            VarNodeValuation::Mult(ab) => {
                let pair = ab.as_ref();
                let a = pair.0.simplify();
                let b = pair.1.simplify();

                // Const * Const = Const
                if let (Some(av), Some(bv)) = (a.as_const(), b.as_const()) {
                    if let VarNodeValuation::Const(vn) = &a {
                        let size = vn.size;
                        return Self::make_const(av.wrapping_mul(bv), size);
                    }
                }

                // x * Const(0) = Const(0)
                if let Some(0) = b.as_const() {
                    return b;
                }
                if let Some(0) = a.as_const() {
                    return a;
                }

                // x * Const(1) = x
                if let Some(1) = b.as_const() {
                    return a;
                }
                if let Some(1) = a.as_const() {
                    return b;
                }

                VarNodeValuation::Mult(Arc::new((a, b)))
            }

            // Bitwise operations
            VarNodeValuation::BitAnd(ab) => {
                let pair = ab.as_ref();
                let a = pair.0.simplify();
                let b = pair.1.simplify();

                // Const & Const = Const
                if let (Some(av), Some(bv)) = (a.as_const(), b.as_const()) {
                    if let VarNodeValuation::Const(vn) = &a {
                        let size = vn.size;
                        return Self::make_const(av & bv, size);
                    }
                }

                // x & Const(0) = Const(0)
                if let Some(0) = b.as_const() {
                    return b;
                }
                if let Some(0) = a.as_const() {
                    return a;
                }

                VarNodeValuation::BitAnd(Arc::new((a, b)))
            }

            VarNodeValuation::BitOr(ab) => {
                let pair = ab.as_ref();
                let a = pair.0.simplify();
                let b = pair.1.simplify();

                // Const | Const = Const
                if let (Some(av), Some(bv)) = (a.as_const(), b.as_const()) {
                    if let VarNodeValuation::Const(vn) = &a {
                        let size = vn.size;
                        return Self::make_const(av | bv, size);
                    }
                }

                // x | Const(0) = x
                if let Some(0) = b.as_const() {
                    return a;
                }
                if let Some(0) = a.as_const() {
                    return b;
                }

                VarNodeValuation::BitOr(Arc::new((a, b)))
            }

            VarNodeValuation::BitXor(ab) => {
                let pair = ab.as_ref();
                let a = pair.0.simplify();
                let b = pair.1.simplify();

                // Const ^ Const = Const
                if let (Some(av), Some(bv)) = (a.as_const(), b.as_const()) {
                    if let VarNodeValuation::Const(vn) = &a {
                        let size = vn.size;
                        return Self::make_const(av ^ bv, size);
                    }
                }

                // x ^ Const(0) = x
                if let Some(0) = b.as_const() {
                    return a;
                }
                if let Some(0) = a.as_const() {
                    return b;
                }

                VarNodeValuation::BitXor(Arc::new((a, b)))
            }

            VarNodeValuation::BitNegate(a) => {
                let a_s = a.as_ref().simplify();

                // ~Const = Const
                if let Some(av) = a_s.as_const() {
                    if let VarNodeValuation::Const(vn) = &a_s {
                        let size = vn.size;
                        let mask = if size >= 8 {
                            u64::MAX
                        } else {
                            (1u64 << (size * 8)) - 1
                        };
                        return Self::make_const(!av & mask, size);
                    }
                }

                VarNodeValuation::BitNegate(Arc::new(a_s))
            }

            VarNodeValuation::Load(a) => {
                let a_s = a.as_ref().simplify();
                VarNodeValuation::Load(Arc::new(a_s))
            }

            VarNodeValuation::Or(ab) => {
                let pair = ab.as_ref();
                let a = pair.0.simplify();
                let b = pair.1.simplify();

                // Const || Const = Const (approximate by folding identical exprs)
                if a == b {
                    return a;
                }

                VarNodeValuation::Or(Arc::new((a, b)))
            }

            // Entry, Const, and Top don't need simplification
            VarNodeValuation::Entry(vn) => VarNodeValuation::Entry(vn.clone()),
            VarNodeValuation::Const(vn) => VarNodeValuation::Const(vn.clone()),
            VarNodeValuation::Top => VarNodeValuation::Top,
        }
    }
}

impl JingleDisplayable for VarNodeValuation {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        match self {
            VarNodeValuation::Entry(vn) => write!(f, "Entry({})", vn.display(info)),
            VarNodeValuation::Const(vn) => write!(f, "{}", vn.display(info)),
            VarNodeValuation::Mult(ab) => {
                let pair = ab.as_ref();
                write!(f, "({}*{})", pair.0.display(info), pair.1.display(info))
            }
            VarNodeValuation::Add(ab) => {
                let pair = ab.as_ref();
                write!(f, "({}+{})", pair.0.display(info), pair.1.display(info))
            }
            VarNodeValuation::Sub(ab) => {
                let pair = ab.as_ref();
                write!(f, "({}-{})", pair.0.display(info), pair.1.display(info))
            }
            VarNodeValuation::BitAnd(ab) => {
                let pair = ab.as_ref();
                write!(f, "({}&{})", pair.0.display(info), pair.1.display(info))
            }
            VarNodeValuation::BitOr(ab) => {
                let pair = ab.as_ref();
                write!(f, "({}|{})", pair.0.display(info), pair.1.display(info))
            }
            VarNodeValuation::BitXor(ab) => {
                let pair = ab.as_ref();
                write!(f, "({}^{})", pair.0.display(info), pair.1.display(info))
            }
            VarNodeValuation::BitNegate(a) => write!(f, "(~{})", a.display(info)),
            VarNodeValuation::Or(ab) => {
                let pair = ab.as_ref();
                write!(f, "({}||{})", pair.0.display(info), pair.1.display(info))
            }
            VarNodeValuation::Load(a) => write!(f, "Load({})", a.display(info)),
            VarNodeValuation::Top => write!(f, "⊤"),
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
pub struct DirectValuationState {
    written_locations: VarNodeMap<VarNodeValuation>,
    arch_info: SleighArchInfo,
    /// Merge behavior controlling how conflicting valuations are handled during `join`.
    merge_behavior: MergeBehavior,
}

impl Hash for DirectValuationState {
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

impl Display for DirectValuationState {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        let hash_value = hasher.finish();
        write!(f, "Hash({:016x})", hash_value)
    }
}

impl DirectValuationState {
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

    pub fn get_value(&self, varnode: &VarNode) -> Option<&VarNodeValuation> {
        self.written_locations.get(varnode)
    }

    pub fn written_locations(&self) -> &VarNodeMap<VarNodeValuation> {
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
                                VarNodeValuation::Const(input.clone())
                            } else {
                                VarNodeValuation::from_varnode_or_entry(self, input)
                            }
                        }

                        // Adds (treat many boolean/bitwise ops as Add/Or/Xor approximations)
                        PcodeOperation::IntAdd { input0, input1, .. } => {
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::Add(Arc::new((a, b)))
                        }

                        PcodeOperation::IntSub { input0, input1, .. } => {
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::Sub(Arc::new((a, b)))
                        }

                        PcodeOperation::IntMult { input0, input1, .. } => {
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::Mult(Arc::new((a, b)))
                        }

                        // Bitwise operations
                        PcodeOperation::IntAnd { input0, input1, .. }
                        | PcodeOperation::BoolAnd { input0, input1, .. } => {
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::BitAnd(Arc::new((a, b)))
                        }

                        PcodeOperation::IntXor { input0, input1, .. }
                        | PcodeOperation::BoolXor { input0, input1, .. } => {
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::BitXor(Arc::new((a, b)))
                        }

                        PcodeOperation::IntOr { input0, input1, .. }
                        | PcodeOperation::BoolOr { input0, input1, .. } => {
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::BitOr(Arc::new((a, b)))
                        }
                        PcodeOperation::IntLeftShift { input0, input1, .. }
                        | PcodeOperation::IntRightShift { input0, input1, .. }
                        | PcodeOperation::IntSignedRightShift { input0, input1, .. } => {
                            // Approximate shifts as an Add of the operands (conservative symbolic form)
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::Add(Arc::new((a, b)))
                        }

                        PcodeOperation::IntNegate { input, .. } => {
                            // Represent negate as Sub(Const(0), input)
                            let zero = VarNode {
                                space_index: VarNode::CONST_SPACE_INDEX,
                                offset: 0,
                                size: input.size,
                            };
                            let a = VarNodeValuation::Const(zero);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input);
                            VarNodeValuation::Sub(Arc::new((a, b)))
                        }

                        PcodeOperation::Int2Comp { input, .. } => {
                            // Approximate two's complement by bit-negation
                            let a = VarNodeValuation::from_varnode_or_entry(self, input);
                            VarNodeValuation::BitNegate(Arc::new(a))
                        }

                        // Load - track pointer expression
                        PcodeOperation::Load { input, .. } => {
                            let ptr = &input.pointer_location;
                            let pv = if ptr.space_index == VarNode::CONST_SPACE_INDEX {
                                VarNodeValuation::Const(ptr.clone())
                            } else {
                                VarNodeValuation::from_varnode_or_entry(self, ptr)
                            };
                            VarNodeValuation::Load(Arc::new(pv))
                        }

                        // Casts/extensions - preserve symbolic value
                        PcodeOperation::IntSExt { input, .. }
                        | PcodeOperation::IntZExt { input, .. } => {
                            VarNodeValuation::from_varnode_or_entry(self, input)
                        }

                        // Default: be conservative and mark as Top
                        _ => VarNodeValuation::Top,
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

impl PartialOrd for VarNodeValuation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

impl JoinSemiLattice for VarNodeValuation {
    fn join(&mut self, _other: &Self) {}
}

impl PartialOrd for DirectValuationState {
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

impl JoinSemiLattice for DirectValuationState {
    fn join(&mut self, other: &Self) {
        // For each varnode present in `other`:
        // - if present in self with same valuation -> keep
        // - if present in self with different valuation -> combine according to merge_behavior
        // - if absent in self -> clone from other
        for (key, other_val) in other.written_locations.iter() {
            match self.written_locations.get_mut(key) {
                Some(my_val) => {
                    if my_val == &VarNodeValuation::Top || other_val == &VarNodeValuation::Top {
                        *my_val = VarNodeValuation::Top;
                    } else if my_val != other_val {
                        match self.merge_behavior {
                            MergeBehavior::Or => {
                                // create Or(...) of the two, then simplify the result
                                let combined = VarNodeValuation::Or(Arc::new((
                                    my_val.clone(),
                                    other_val.clone(),
                                )));
                                *my_val = combined.simplify();
                            }
                            MergeBehavior::Top => {
                                // converge differing values to Top (less precise, but useful when not unwinding locations)
                                *my_val = VarNodeValuation::Top;
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

pub struct DirectValuationAnalysis {
    arch_info: SleighArchInfo,
    /// Default merge behavior for states produced by this analysis.
    merge_behavior: MergeBehavior,
}

impl DirectValuationAnalysis {
    /// Create with the default merge behavior (`Or`).
    pub fn new(arch_info: SleighArchInfo, merge_behavior: MergeBehavior) -> Self {
        Self {
            arch_info,
            merge_behavior,
        }
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
            written_locations: VarNodeMap::new(),
            arch_info: c.arch_info.clone(),
            merge_behavior: c.merge_behavior,
        }
    }
}
