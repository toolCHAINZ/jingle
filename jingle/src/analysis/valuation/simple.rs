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

use crate::analysis::valuation::ast::{Add, Entry, Load, Mul, Or, SimpleValue, Sub};

/// Re-export the AST SimpleValue so external modules that import through
/// `analysis::valuation::simple::SimpleValue` keep working.

/// Provide helper to create a SimpleValue from a VarNode or Entry located
/// in a SimpleValuationState. This mirrors the prior behavior that used
/// a locally-defined `SimpleValue` enum.
impl SimpleValue {
    /// Resolve a VarNode to an existing valuation in the state, a Const, or an Entry.
    pub fn from_varnode_or_entry(state: &SimpleValuationState, vn: &VarNode) -> Self {
        if vn.space_index == VarNode::CONST_SPACE_INDEX {
            SimpleValue::Const(vn.offset as i64)
        } else if let Some(v) = state.written_locations.get(vn) {
            v.clone()
        } else {
            SimpleValue::Entry(Entry(Intern::new(vn.clone())))
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
///
/// The state stores a map of written varnodes -> SimpleValue (the AST representation).
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SimpleValuationState {
    written_locations: VarNodeMap<SimpleValue>,
    arch_info: SleighArchInfo,
    /// Merge behavior controlling how conflicting valuations are handled during `join`.
    merge_behavior: MergeBehavior,
}

impl Hash for SimpleValuationState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // `VarNodeMap` stores keys in sorted order; iterate deterministically.
        for (vn, val) in self.written_locations.items() {
            vn.hash(state);
            val.hash(state);
        }
        // include merge behavior and arch info in the hash
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
        for (vn, val) in self.written_locations.items() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            // Use the JingleDisplay implementations for VarNode and SimpleValue
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

    pub fn get_value(&self, varnode: &VarNode) -> Option<&SimpleValue> {
        self.written_locations.get(varnode)
    }

    pub fn written_locations(&self) -> &VarNodeMap<SimpleValue> {
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
                                SimpleValue::Const(input.offset as i64)
                            } else {
                                SimpleValue::from_varnode_or_entry(self, input)
                            }
                        }

                        PcodeOperation::IntAdd { input0, input1, .. } => {
                            let a = SimpleValue::from_varnode_or_entry(self, input0);
                            let b = SimpleValue::from_varnode_or_entry(self, input1);
                            SimpleValue::add(a, b)
                        }

                        PcodeOperation::IntSub { input0, input1, .. } => {
                            let a = SimpleValue::from_varnode_or_entry(self, input0);
                            let b = SimpleValue::from_varnode_or_entry(self, input1);
                            SimpleValue::sub(a, b)
                        }

                        PcodeOperation::IntMult { input0, input1, .. } => {
                            let a = SimpleValue::from_varnode_or_entry(self, input0);
                            let b = SimpleValue::from_varnode_or_entry(self, input1);
                            SimpleValue::mul(a, b)
                        }

                        // Bool/bit operations - approximate/record
                        PcodeOperation::IntAnd { input0, input1, .. }
                        | PcodeOperation::BoolAnd { input0, input1, .. } => {
                            // TODO: reintroduce BitAnd when needed
                            let a = SimpleValue::from_varnode_or_entry(self, input0);
                            let b = SimpleValue::from_varnode_or_entry(self, input1);
                            // represent as Or fallback for now if desired, otherwise Top
                            SimpleValue::Top
                        }

                        PcodeOperation::IntXor { input0, input1, .. }
                        | PcodeOperation::BoolXor { input0, input1, .. } => {
                            let a = SimpleValue::from_varnode_or_entry(self, input0);
                            let b = SimpleValue::from_varnode_or_entry(self, input1);
                            // not supported in this minimal AST; mark Top
                            SimpleValue::Top
                        }

                        PcodeOperation::IntOr { input0, input1, .. }
                        | PcodeOperation::BoolOr { input0, input1, .. } => {
                            let a = SimpleValue::from_varnode_or_entry(self, input0);
                            let b = SimpleValue::from_varnode_or_entry(self, input1);
                            SimpleValue::or(a, b)
                        }

                        PcodeOperation::IntLeftShift { input0, input1, .. }
                        | PcodeOperation::IntRightShift { input0, input1, .. }
                        | PcodeOperation::IntSignedRightShift { input0, input1, .. } => {
                            // Approximate shifts as an Add of the operands (conservative symbolic form)
                            let a = SimpleValue::from_varnode_or_entry(self, input0);
                            let b = SimpleValue::from_varnode_or_entry(self, input1);
                            SimpleValue::add(a, b)
                        }

                        PcodeOperation::IntNegate { input, .. } => {
                            // Represent negate as Sub(Const(0), input)
                            let a = SimpleValue::const_(0);
                            let b = SimpleValue::from_varnode_or_entry(self, input);
                            SimpleValue::sub(a, b)
                        }

                        PcodeOperation::Int2Comp { input, .. } => {
                            // approximate as Top or BitNegate when reintroduced
                            let a = SimpleValue::from_varnode_or_entry(self, input);
                            SimpleValue::Top
                        }

                        // Load - track pointer expression
                        PcodeOperation::Load { input, .. } => {
                            let ptr = &input.pointer_location;
                            let pv = if ptr.space_index == VarNode::CONST_SPACE_INDEX {
                                tracing::warn!("Constant address used in indirect load");
                                SimpleValue::const_(ptr.offset as i64)
                            } else {
                                SimpleValue::from_varnode_or_entry(self, ptr)
                            };
                            SimpleValue::load(pv)
                        }

                        // Casts/extensions - preserve symbolic value
                        PcodeOperation::IntSExt { input, .. }
                        | PcodeOperation::IntZExt { input, .. } => {
                            SimpleValue::from_varnode_or_entry(self, input)
                        }

                        // Default: be conservative and mark as Top
                        _ => SimpleValue::Top,
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

        // Clear internal-space varnodes on control-flow to non-const destinations
        match op {
            PcodeOperation::Branch { input } | PcodeOperation::CBranch { input0: input, .. } => {
                if input.space_index != VarNode::CONST_SPACE_INDEX {
                    // VarNodeMap doesn't provide `retain`; collect keys to remove and remove them.
                    let mut to_remove: Vec<VarNode> = Vec::new();
                    for (vn, _) in new_state.written_locations.items() {
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
                for (vn, _) in new_state.written_locations.items() {
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

impl PartialOrd for SimpleValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

impl JoinSemiLattice for SimpleValue {
    fn join(&mut self, _other: &Self) {}
}

impl PartialOrd for SimpleValuationState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Make states comparable only when they have the same keys and identical valuations.
        if self.written_locations.len() != other.written_locations.len() {
            return None;
        }

        for (key, val) in self.written_locations.items() {
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
        for (key, other_val) in other.written_locations.items() {
            match self.written_locations.get_mut(key) {
                Some(my_val) => {
                    if my_val == &SimpleValue::Top || other_val == &SimpleValue::Top {
                        *my_val = SimpleValue::Top;
                    } else if my_val != other_val {
                        match self.merge_behavior {
                            MergeBehavior::Or => {
                                // create Or(...) of the two, then simplify the result
                                let combined = SimpleValue::or(my_val.clone(), other_val.clone());
                                *my_val = combined.simplify();
                            }
                            MergeBehavior::Top => {
                                // converge differing values to Top (less precise)
                                *my_val = SimpleValue::Top;
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

/// Analysis entrypoint using the SimpleValue AST-based representation.
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
