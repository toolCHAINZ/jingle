pub mod valuation;
pub mod value;

use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::residue::EmptyResidue;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::valuation::simple::valuation::SimpleValuation;
use crate::analysis::varnode_map::VarNodeMap;
use crate::display::JingleDisplay;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{GeneralizedVarNode, PcodeOperation, SleighArchInfo, SpaceType, VarNode};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};

use crate::analysis::valuation::simple::value::SimpleValue;

/// How to merge conflicting valuations for a single varnode when joining states.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MergeBehavior {
    /// Combine differing valuations into an `Or(...)` expression (higher precision).
    Or,
    /// Converge differing valuations to `Top` (lower precision).
    Top,
}

/// State for the valuation CPA. Stores a `SimpleValuation` which contains both
/// direct and indirect write maps.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SimpleValuationState {
    valuation: SimpleValuation,
    arch_info: SleighArchInfo,
    /// Merge behavior controlling how conflicting valuations are handled during `join`.
    merge_behavior: MergeBehavior,
}

impl Hash for SimpleValuationState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash direct writes (VarNodeMap provides deterministic ordering).
        for (vn, val) in self.valuation.direct_writes.items() {
            vn.hash(state);
            val.hash(state);
        }

        // Hash indirect writes deterministically by sorting keys' debug representations.
        // This keeps the hash stable across runs.
        let mut kvs: Vec<_> = self.valuation.indirect_writes.iter().collect();
        kvs.sort_by(|a, b| format!("{:?}", a.0).cmp(&format!("{:?}", b.0)));
        for (k, v) in kvs {
            k.hash(state);
            v.hash(state);
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
        write!(f, "SimpleValuationState {{")?;
        let mut first = true;

        // Direct writes (vn -> val)
        for (vn, val) in self.valuation.direct_writes.items() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{} = {}", vn.display(info), val.display(info))?;
        }

        // Indirect writes ([ptr_expr] -> val)
        for (ptr, val) in &self.valuation.indirect_writes {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "[{}] = {}", ptr.display(info), val.display(info))?;
        }

        write!(f, "}}")?;
        Ok(())
    }
}

impl SimpleValuationState {
    /// Create a new state with the default merge behavior of `Or`.
    pub fn new(arch_info: SleighArchInfo) -> Self {
        Self {
            valuation: SimpleValuation::new(),
            arch_info,
            merge_behavior: MergeBehavior::Or,
        }
    }

    /// Create a new state specifying the desired merge behavior.
    pub fn new_with_behavior(arch_info: SleighArchInfo, merge_behavior: MergeBehavior) -> Self {
        Self {
            valuation: SimpleValuation::new(),
            arch_info,
            merge_behavior,
        }
    }

    pub fn get_value(&self, varnode: &VarNode) -> Option<&SimpleValue> {
        self.valuation.direct_writes.get(varnode)
    }

    pub fn written_locations(&self) -> &VarNodeMap<SimpleValue> {
        &self.valuation.direct_writes
    }

    pub fn valuation(&self) -> &SimpleValuation {
        &self.valuation
    }

    /// Transfer function: build symbolic valuations for pcode operations.
    /// This returns a new state (functional) instead of mutating in place.
    fn transfer_impl(&self, op: &PcodeOperation) -> Self {
        let mut new_state = self.clone();

        // Match on the operation. Handle stores (indirect) and direct-output ops.
        match op {
            // Store: record pointer -> value in indirect_writes
            PcodeOperation::Store { output, input } => {
                let ptr = &output.pointer_location;
                let val = if input.space_index == VarNode::CONST_SPACE_INDEX {
                    SimpleValue::const_(input.offset as i64)
                } else {
                    SimpleValue::from_varnode_or_entry(self, input)
                };

                let pv = SimpleValue::from_varnode_or_entry(self, ptr);
                new_state
                    .valuation
                    .indirect_writes
                    .insert(pv.simplify(), val.simplify());
            }

            // Copy
            PcodeOperation::Copy { input, .. } => {
                let result = if input.space_index == VarNode::CONST_SPACE_INDEX {
                    SimpleValue::const_(input.offset as i64)
                } else {
                    SimpleValue::from_varnode_or_entry(self, input)
                };
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .direct_writes
                        .insert(output_vn, result.simplify());
                }
            }

            PcodeOperation::IntAdd { input0, input1, .. } => {
                let a = SimpleValue::from_varnode_or_entry(self, input0);
                let b = SimpleValue::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .direct_writes
                        .insert(output_vn, (a + b).simplify());
                }
            }

            PcodeOperation::IntSub { input0, input1, .. } => {
                let a = SimpleValue::from_varnode_or_entry(self, input0);
                let b = SimpleValue::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .direct_writes
                        .insert(output_vn, (a - b).simplify());
                }
            }

            PcodeOperation::IntMult { input0, input1, .. } => {
                let a = SimpleValue::from_varnode_or_entry(self, input0);
                let b = SimpleValue::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .direct_writes
                        .insert(output_vn, (a * b).simplify());
                }
            }

            PcodeOperation::IntOr { input0, input1, .. }
            | PcodeOperation::BoolOr { input0, input1, .. } => {
                let a = SimpleValue::from_varnode_or_entry(self, input0);
                let b = SimpleValue::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .direct_writes
                        .insert(output_vn, SimpleValue::or(a, b).simplify());
                }
            }

            // Approximate shifts as addition (conservative)
            PcodeOperation::IntLeftShift { input0, input1, .. }
            | PcodeOperation::IntRightShift { input0, input1, .. }
            | PcodeOperation::IntSignedRightShift { input0, input1, .. } => {
                let a = SimpleValue::from_varnode_or_entry(self, input0);
                let b = SimpleValue::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .direct_writes
                        .insert(output_vn, (a + b).simplify());
                }
            }

            PcodeOperation::IntNegate { input, .. } => {
                let a = SimpleValue::const_(0);
                let b = SimpleValue::from_varnode_or_entry(self, input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .direct_writes
                        .insert(output_vn, (a - b).simplify());
                }
            }

            PcodeOperation::Int2Comp { .. } => {
                // conservative
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .direct_writes
                        .insert(output_vn, SimpleValue::Top);
                }
            }

            PcodeOperation::Load { input, .. } => {
                let ptr = &input.pointer_location;

                // Non-constant pointer: if we have an indirect write recorded for this
                // pointer expression, use that stored value directly; otherwise emit Load(...)
                let pv = SimpleValue::from_varnode_or_entry(self, ptr);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    if let Some(v) = self.valuation.indirect_writes.get(&pv.simplify()) {
                        new_state
                            .valuation
                            .direct_writes
                            .insert(output_vn, v.clone());
                    } else {
                        new_state
                            .valuation
                            .direct_writes
                            .insert(output_vn, SimpleValue::load(pv).simplify());
                    }
                }
            }

            PcodeOperation::IntSExt { input, .. } | PcodeOperation::IntZExt { input, .. } => {
                let v = SimpleValue::from_varnode_or_entry(self, input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .direct_writes
                        .insert(output_vn, v.simplify());
                }
            }

            // Other operations we don't model produce no tracked writes here.
            _ => {}
        }

        // Clear internal-space varnodes on control-flow to non-const destinations (same policy as direct_valuation.rs)
        match op {
            PcodeOperation::Branch { input } | PcodeOperation::CBranch { input0: input, .. } => {
                if input.space_index != VarNode::CONST_SPACE_INDEX {
                    // VarNodeMap doesn't provide `retain`; collect keys to remove and remove them.
                    let mut to_remove: Vec<VarNode> = Vec::new();
                    for (vn, _) in new_state.valuation.direct_writes.items() {
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
                        new_state.valuation.direct_writes.remove(&k);
                    }
                }
            }
            PcodeOperation::BranchInd { .. } | PcodeOperation::CallInd { .. } => {
                // Similar retain behavior as above for branch-indirect.
                let mut to_remove: Vec<VarNode> = Vec::new();
                for (vn, _) in new_state.valuation.direct_writes.items() {
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
                    new_state.valuation.direct_writes.remove(&k);
                }
            }
            _ => {}
        }

        new_state
    }
}

impl PartialOrd for SimpleValuationState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Make states comparable only when they have the same direct keys and identical valuations.
        if self.valuation.direct_writes.len() != other.valuation.direct_writes.len() {
            return None;
        }

        for (key, val) in self.valuation.direct_writes.items() {
            match other.valuation.direct_writes.get(key) {
                Some(other_val) => {
                    if val != other_val {
                        return None;
                    }
                }
                None => return None,
            }
        }

        // Also require indirect maps to be identical for comparability.
        if self.valuation.indirect_writes.len() != other.valuation.indirect_writes.len() {
            return None;
        }
        for (k, v) in &self.valuation.indirect_writes {
            match other.valuation.indirect_writes.get(k) {
                Some(ov) => {
                    if v != ov {
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
        // Merge direct writes
        for (key, other_val) in other.valuation.direct_writes.items() {
            match self.valuation.direct_writes.get_mut(key) {
                Some(my_val) => {
                    if my_val == &SimpleValue::Top || other_val == &SimpleValue::Top {
                        *my_val = SimpleValue::Top;
                    } else if my_val != other_val {
                        match self.merge_behavior {
                            MergeBehavior::Or => {
                                let combined = SimpleValue::or(my_val.clone(), other_val.clone());
                                *my_val = combined.simplify();
                            }
                            MergeBehavior::Top => {
                                *my_val = SimpleValue::Top;
                            }
                        }
                    }
                }
                None => {
                    self.valuation
                        .direct_writes
                        .insert(key.clone(), other_val.clone());
                }
            }
        }

        // Merge indirect writes (pointer -> value)
        for (key, other_val) in &other.valuation.indirect_writes {
            match self.valuation.indirect_writes.get_mut(key) {
                Some(my_val) => {
                    if my_val == &SimpleValue::Top || other_val == &SimpleValue::Top {
                        *my_val = SimpleValue::Top;
                    } else if my_val != other_val {
                        match self.merge_behavior {
                            MergeBehavior::Or => {
                                let combined = SimpleValue::or(my_val.clone(), other_val.clone());
                                *my_val = combined.simplify();
                            }
                            MergeBehavior::Top => {
                                *my_val = SimpleValue::Top;
                            }
                        }
                    }
                }
                None => {
                    self.valuation
                        .indirect_writes
                        .insert(key.clone(), other_val.clone());
                }
            }
        }
    }
}

impl AbstractState for SimpleValuationState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        // Reuse the lattice join helper if available; otherwise, perform join and return a conservative outcome.
        // Many CPAs provide `merge_join` via a helper trait in this codebase; call it if present.
        // Fallback: perform a join (mutating self) and report that we merged.
        // We'll try to call `merge_join` as in the original design.
        self.merge_join(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        // Defer to the standard stop predicate helper if available.
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
            valuation: SimpleValuation::new(),
            arch_info: c.arch_info.clone(),
            merge_behavior: c.merge_behavior,
        }
    }
}
