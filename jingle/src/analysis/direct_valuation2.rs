use crate::analysis::Analysis;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::residue::EmptyResidue;
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, StateDisplay, Successor};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::varnode_map::VarNodeMap;
use crate::display::JingleDisplayable;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{GeneralizedVarNode, PcodeOperation, SleighArchInfo, SpaceType, VarNode};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};

/// Symbolic valuation built from varnodes and constants.
///
/// This valuation intentionally does not include a Top element. Unknown or conflicting
/// information is handled at the state join level by reverting the varnode to the
/// `Entry(varnode)` form. This is acceptable for unwound / bounded analyses.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum VarNodeValuation {
    Entry(VarNode),
    Const(VarNode),
    Mult(Box<VarNodeValuation>, Box<VarNodeValuation>),
    Add(Box<VarNodeValuation>, Box<VarNodeValuation>),
    Sub(Box<VarNodeValuation>, Box<VarNodeValuation>),
    BitAnd(Box<VarNodeValuation>, Box<VarNodeValuation>),
    BitOr(Box<VarNodeValuation>, Box<VarNodeValuation>),
    BitXor(Box<VarNodeValuation>, Box<VarNodeValuation>),
    BitNegate(Box<VarNodeValuation>),
    Or(Box<VarNodeValuation>, Box<VarNodeValuation>),
    Load(Box<VarNodeValuation>),
    Top,
}

impl VarNodeValuation {
    fn from_varnode_or_entry(state: &DirectValuation2State, vn: &VarNode) -> Self {
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
    fn as_const(&self) -> Option<u64> {
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
    fn simplify(&mut self) {
        match self {
            // Arithmetic operations
            VarNodeValuation::Add(a, b) => {
                // First simplify children
                a.simplify();
                b.simplify();

                // Const + Const = Const
                if let (Some(av), Some(bv)) = (a.as_const(), b.as_const()) {
                    if let VarNodeValuation::Const(vn) = a.as_ref() {
                        let size = vn.size;
                        *self = Self::make_const(av.wrapping_add(bv), size);
                        return;
                    }
                }

                // Flatten nested Add with Const: (Add(x, Const(c1)) + Const(c2)) -> Add(x, Const(c1+c2))
                if let (VarNodeValuation::Add(_inner_a, inner_b), Some(bv)) =
                    (a.as_mut(), b.as_const())
                {
                    if let Some(inner_bv) = inner_b.as_const() {
                        if let VarNodeValuation::Const(vn) = inner_b.as_ref() {
                            let size = vn.size;
                            **inner_b = Self::make_const(inner_bv.wrapping_add(bv), size);
                            *self = (**a).clone();
                            return;
                        }
                    }
                }

                // Symmetric case: Const(c1) + Add(x, Const(c2)) -> Add(x, Const(c1+c2))
                if let (Some(av), VarNodeValuation::Add(_inner_a, inner_b)) =
                    (a.as_const(), b.as_mut())
                {
                    if let Some(inner_bv) = inner_b.as_const() {
                        if let VarNodeValuation::Const(vn) = inner_b.as_ref() {
                            let size = vn.size;
                            **inner_b = Self::make_const(av.wrapping_add(inner_bv), size);
                            *self = (**b).clone();
                            return;
                        }
                    }
                }
            }

            VarNodeValuation::Sub(a, b) => {
                a.simplify();
                b.simplify();

                // Const - Const = Const
                if let (Some(av), Some(bv)) = (a.as_const(), b.as_const()) {
                    if let VarNodeValuation::Const(vn) = a.as_ref() {
                        let size = vn.size;
                        *self = Self::make_const(av.wrapping_sub(bv), size);
                        return;
                    }
                }

                // x - Const(0) = x
                if let Some(0) = b.as_const() {
                    *self = (**a).clone();
                    return;
                }
            }

            VarNodeValuation::Mult(a, b) => {
                a.simplify();
                b.simplify();

                // Const * Const = Const
                if let (Some(av), Some(bv)) = (a.as_const(), b.as_const()) {
                    if let VarNodeValuation::Const(vn) = a.as_ref() {
                        let size = vn.size;
                        *self = Self::make_const(av.wrapping_mul(bv), size);
                        return;
                    }
                }

                // x * Const(0) = Const(0)
                if let Some(0) = b.as_const() {
                    *self = (**b).clone();
                    return;
                }
                if let Some(0) = a.as_const() {
                    *self = (**a).clone();
                    return;
                }

                // x * Const(1) = x
                if let Some(1) = b.as_const() {
                    *self = (**a).clone();
                    return;
                }
                if let Some(1) = a.as_const() {
                    *self = (**b).clone();
                    return;
                }
            }

            // Bitwise operations
            VarNodeValuation::BitAnd(a, b) => {
                a.simplify();
                b.simplify();

                // Const & Const = Const
                if let (Some(av), Some(bv)) = (a.as_const(), b.as_const()) {
                    if let VarNodeValuation::Const(vn) = a.as_ref() {
                        let size = vn.size;
                        *self = Self::make_const(av & bv, size);
                        return;
                    }
                }

                // x & Const(0) = Const(0)
                if let Some(0) = b.as_const() {
                    *self = (**b).clone();
                    return;
                }
                if let Some(0) = a.as_const() {
                    *self = (**a).clone();
                    return;
                }
            }

            VarNodeValuation::BitOr(a, b) => {
                a.simplify();
                b.simplify();

                // Const | Const = Const
                if let (Some(av), Some(bv)) = (a.as_const(), b.as_const()) {
                    if let VarNodeValuation::Const(vn) = a.as_ref() {
                        let size = vn.size;
                        *self = Self::make_const(av | bv, size);
                        return;
                    }
                }

                // x | Const(0) = x
                if let Some(0) = b.as_const() {
                    *self = (**a).clone();
                    return;
                }
                if let Some(0) = a.as_const() {
                    *self = (**b).clone();
                    return;
                }
            }

            VarNodeValuation::BitXor(a, b) => {
                a.simplify();
                b.simplify();

                // Const ^ Const = Const
                if let (Some(av), Some(bv)) = (a.as_const(), b.as_const()) {
                    if let VarNodeValuation::Const(vn) = a.as_ref() {
                        let size = vn.size;
                        *self = Self::make_const(av ^ bv, size);
                        return;
                    }
                }

                // x ^ Const(0) = x
                if let Some(0) = b.as_const() {
                    *self = (**a).clone();
                    return;
                }
                if let Some(0) = a.as_const() {
                    *self = (**b).clone();
                    return;
                }
            }

            VarNodeValuation::BitNegate(a) => {
                a.simplify();

                // ~Const = Const
                if let Some(av) = a.as_const() {
                    if let VarNodeValuation::Const(vn) = a.as_ref() {
                        let size = vn.size;
                        let mask = if size >= 8 {
                            u64::MAX
                        } else {
                            (1u64 << (size * 8)) - 1
                        };
                        *self = Self::make_const(!av & mask, size);
                        return;
                    }
                }
            }

            VarNodeValuation::Load(a) => {
                a.simplify();
            }

            VarNodeValuation::Or(a, b) => {
                a.simplify();
                b.simplify();

                // Const || Const = Const
                if a == b {
                    *self = (**a).clone();
                    return;
                }
            }

            // Entry, Const, and Top don't need simplification
            VarNodeValuation::Entry(_) | VarNodeValuation::Const(_) | VarNodeValuation::Top => {}
        }
    }
}

impl JingleDisplayable for VarNodeValuation {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        match self {
            VarNodeValuation::Entry(vn) => write!(f, "Entry({})", vn.display(info)),
            VarNodeValuation::Const(vn) => write!(f, "Const({})", vn.display(info)),
            VarNodeValuation::Mult(a, b) => write!(f, "({}*{})", a.display(info), b.display(info)),
            VarNodeValuation::Add(a, b) => write!(f, "({}+{})", a.display(info), b.display(info)),
            VarNodeValuation::Sub(a, b) => write!(f, "({}-{})", a.display(info), b.display(info)),
            VarNodeValuation::BitAnd(a, b) => {
                write!(f, "({}&{})", a.display(info), b.display(info))
            }
            VarNodeValuation::BitOr(a, b) => write!(f, "({}|{})", a.display(info), b.display(info)),
            VarNodeValuation::BitXor(a, b) => {
                write!(f, "({}^{})", a.display(info), b.display(info))
            }
            VarNodeValuation::BitNegate(a) => write!(f, "(~{})", a.display(info)),
            VarNodeValuation::Or(a, b) => write!(f, "({}||{})", a.display(info), b.display(info)),
            VarNodeValuation::Load(a) => write!(f, "Load({})", a.display(info)),
            VarNodeValuation::Top => write!(f, "⊤"),
        }
    }
}

/// State for the VarNodeValuation-based direct valuation CPA.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DirectValuation2State {
    written_locations: VarNodeMap<VarNodeValuation>,
    arch_info: SleighArchInfo,
}

impl Hash for DirectValuation2State {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // `VarNodeMap` stores keys in sorted order; iterate deterministically.
        for (vn, val) in self.written_locations.iter() {
            vn.hash(state);
            val.hash(state);
        }
        self.arch_info.hash(state);
    }
}

impl StateDisplay for DirectValuation2State {
    fn fmt_state(&self, f: &mut Formatter<'_>) -> FmtResult {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        let hash_value = hasher.finish();
        write!(f, "Hash({:016x})", hash_value)
    }
}

impl DirectValuation2State {
    pub fn new(arch_info: SleighArchInfo) -> Self {
        Self {
            written_locations: VarNodeMap::new(),
            arch_info,
        }
    }

    pub fn get_value(&self, varnode: &VarNode) -> Option<&VarNodeValuation> {
        self.written_locations.get(varnode)
    }

    pub fn written_locations(&self) -> &VarNodeMap<VarNodeValuation> {
        &self.written_locations
    }

    /// Transfer function: build symbolic valuations for pcode operations.
    fn transfer_impl(&self, op: &PcodeOperation) -> Self {
        let mut new_state = self.clone();

        if let Some(output) = op.output() {
            match output {
                GeneralizedVarNode::Direct(output_vn) => {
                    let mut result_val = match op {
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
                            VarNodeValuation::Add(Box::new(a), Box::new(b))
                        }

                        PcodeOperation::IntSub { input0, input1, .. } => {
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::Sub(Box::new(a), Box::new(b))
                        }

                        PcodeOperation::IntMult { input0, input1, .. } => {
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::Mult(Box::new(a), Box::new(b))
                        }

                        // Bitwise operations
                        PcodeOperation::IntAnd { input0, input1, .. }
                        | PcodeOperation::BoolAnd { input0, input1, .. } => {
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::BitAnd(Box::new(a), Box::new(b))
                        }

                        PcodeOperation::IntXor { input0, input1, .. }
                        | PcodeOperation::BoolXor { input0, input1, .. } => {
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::BitXor(Box::new(a), Box::new(b))
                        }

                        PcodeOperation::IntOr { input0, input1, .. }
                        | PcodeOperation::BoolOr { input0, input1, .. } => {
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::BitOr(Box::new(a), Box::new(b))
                        }
                        PcodeOperation::IntLeftShift { input0, input1, .. }
                        | PcodeOperation::IntRightShift { input0, input1, .. }
                        | PcodeOperation::IntSignedRightShift { input0, input1, .. } => {
                            // Approximate shifts as an Add of the operands (conservative symbolic form)
                            let a = VarNodeValuation::from_varnode_or_entry(self, input0);
                            let b = VarNodeValuation::from_varnode_or_entry(self, input1);
                            VarNodeValuation::Add(Box::new(a), Box::new(b))
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
                            VarNodeValuation::Sub(Box::new(a), Box::new(b))
                        }

                        PcodeOperation::Int2Comp { input, .. } => {
                            // Approximate two's complement by bit-negation
                            let a = VarNodeValuation::from_varnode_or_entry(self, input);
                            VarNodeValuation::BitNegate(Box::new(a))
                        }

                        // Load - track pointer expression
                        PcodeOperation::Load { input, .. } => {
                            let ptr = &input.pointer_location;
                            let pv = if ptr.space_index == VarNode::CONST_SPACE_INDEX {
                                VarNodeValuation::Const(ptr.clone())
                            } else {
                                VarNodeValuation::from_varnode_or_entry(self, ptr)
                            };
                            VarNodeValuation::Load(Box::new(pv))
                        }

                        // Casts/extensions - preserve symbolic value
                        PcodeOperation::IntSExt { input, .. }
                        | PcodeOperation::IntZExt { input, .. } => {
                            VarNodeValuation::from_varnode_or_entry(self, input)
                        }

                        // Default: be conservative and mark as Entry(output_vn)
                        _ => VarNodeValuation::Top,
                    };
                    result_val.simplify();
                    new_state.written_locations.insert(output_vn, result_val);
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
    fn join(&mut self, _other: &Self) {
        // The state-level join will handle conflicts by reverting to Entry(varnode).
        // Individual valuation join is a no-op here.
    }
}

impl PartialOrd for DirectValuation2State {
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

impl JoinSemiLattice for DirectValuation2State {
    fn join(&mut self, other: &Self) {
        // For each varnode present in `other`:
        // - if present in self with same valuation -> keep
        // - if present in self with different valuation -> revert to Entry(varnode)
        // - if absent in self -> clone from other
        for (key, other_val) in other.written_locations.iter() {
            match self.written_locations.get_mut(key) {
                Some(my_val) => {
                    if my_val == &VarNodeValuation::Top || other_val == &VarNodeValuation::Top {
                        *my_val = VarNodeValuation::Top;
                    } else if my_val != other_val {
                        *my_val = VarNodeValuation::Or(
                            Box::new(my_val.clone()),
                            Box::new(other_val.clone()),
                        );
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

impl AbstractState for DirectValuation2State {
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

// Allow use in compound analyses
impl<L :LocationState>
    crate::analysis::compound::Strengthen<L>
    for DirectValuation2State
{
}

pub struct DirectValuation2Analysis {
    arch_info: SleighArchInfo,
}

impl DirectValuation2Analysis {
    pub fn new(arch_info: SleighArchInfo) -> Self {
        Self { arch_info }
    }

}

impl ConfigurableProgramAnalysis for DirectValuation2Analysis {
    type State = DirectValuation2State;
    type Reducer = EmptyResidue<Self::State>;
}

impl Analysis for DirectValuation2Analysis {}

impl IntoState<DirectValuation2Analysis> for ConcretePcodeAddress {
    fn into_state(
        self,
        c: &DirectValuation2Analysis,
    ) -> <DirectValuation2Analysis as ConfigurableProgramAnalysis>::State {
        DirectValuation2State {
            written_locations: VarNodeMap::new(),
            arch_info: c.arch_info.clone(),
        }
    }
}
