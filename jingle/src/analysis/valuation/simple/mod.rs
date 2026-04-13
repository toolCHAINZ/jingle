pub mod valuation;
pub mod value;

use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::residue::EmptyResidue;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::valuation::simple::valuation::ValuationSet;
use crate::analysis::varnode_map::VarNodeMap;
use crate::display::JingleDisplay;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{GeneralizedVarNode, PcodeOperation, SleighArchInfo, SpaceType, VarNode};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};

use crate::analysis::valuation::simple::value::{Load, Value};
use internment::Intern;

/// How to merge conflicting valuations for a single varnode when joining states.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MergeBehavior {
    /// Combine differing valuations into a `Choice(...)` expression (higher precision).
    Choice,
    /// Converge differing valuations to `Top` (lower precision).
    Top,
}

/// State for the valuation CPA. Stores a `ValuationSet` which contains both
/// direct and indirect write maps.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ValuationState {
    valuation: ValuationSet,
    arch_info: SleighArchInfo,
    /// Merge behavior controlling how conflicting valuations are handled during `join`.
    merge_behavior: MergeBehavior,
}

impl AsRef<SleighArchInfo> for ValuationState {
    fn as_ref(&self) -> &SleighArchInfo {
        &self.arch_info
    }
}

impl Display for ValuationState {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        let hash_value = hasher.finish();
        write!(f, "Hash({:016x})", hash_value)
    }
}

impl JingleDisplay for ValuationState {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        // Delegate display to the inner Valuation implementation to avoid duplication.
        self.valuation.fmt_jingle(f, info)
    }
}

impl ValuationState {
    /// Create a new state with the default merge behavior of `Or`.
    pub fn new(arch_info: SleighArchInfo) -> Self {
        Self {
            valuation: ValuationSet::new(),
            arch_info,
            merge_behavior: MergeBehavior::Choice,
        }
    }

    /// Create a new state specifying the desired merge behavior.
    pub fn new_with_behavior(arch_info: SleighArchInfo, merge_behavior: MergeBehavior) -> Self {
        Self {
            valuation: ValuationSet::new(),
            arch_info,
            merge_behavior,
        }
    }

    pub fn get_value(&self, varnode: &VarNode) -> Option<&Value> {
        self.valuation.direct_writes.get(varnode)
    }

    pub fn written_locations(&self) -> &VarNodeMap<Value> {
        &self.valuation.direct_writes
    }

    pub fn valuation(&self) -> &ValuationSet {
        &self.valuation
    }

    /// Transfer function: build symbolic valuations for pcode operations.
    /// This returns a new state (functional) instead of mutating in place.
    fn transfer_impl(&self, op: &PcodeOperation) -> Self {
        let mut new_state = self.clone();

        // Match on the operation. Handle stores (indirect) and direct-output ops.
        match op {
            // Store: record Load(ptr, size) -> value in indirect_writes
            PcodeOperation::Store { output, input } => {
                let ptr = &output.pointer_location();
                let val = Value::from_varnode_or_entry(self, input);
                let pv = Value::from_varnode_or_entry(self, ptr);
                let data_size = input.size();
                let loc = Value::Load(Load(Intern::new(pv.simplify()), data_size));
                new_state.valuation.add(loc, val.simplify());
            }

            // Copy
            PcodeOperation::Copy { input, .. } => {
                let result = if input.is_const() {
                    Value::const_(input.offset() as i64)
                } else {
                    Value::from_varnode_or_entry(self, input)
                };
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, result.simplify());
                }
            }

            PcodeOperation::IntAdd { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, (a + b).simplify());
                }
            }

            PcodeOperation::IntSub { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, (a - b).simplify());
                }
            }

            PcodeOperation::IntXor { input0, input1, .. }
            | PcodeOperation::BoolXor { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, (a ^ b).simplify());
                }
            }

            PcodeOperation::IntMult { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, (a * b).simplify());
                }
            }

            PcodeOperation::IntOr { input0, input1, .. }
            | PcodeOperation::BoolOr { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, (a | b).simplify());
                }
            }

            PcodeOperation::IntAnd { input0, input1, .. }
            | PcodeOperation::BoolAnd { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, (a & b).simplify());
                }
            }

            PcodeOperation::IntLeftShift { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let s = std::cmp::max(a.size(), b.size());
                    let shift_expr = Value::IntLeftShift(value::IntLeftShiftExpr(
                        Intern::new(a),
                        Intern::new(b),
                        s,
                    ));
                    new_state.valuation.add(output_vn, shift_expr.simplify());
                }
            }

            PcodeOperation::IntRightShift { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let s = std::cmp::max(a.size(), b.size());
                    let shift_expr = Value::IntRightShift(value::IntRightShiftExpr(
                        Intern::new(a),
                        Intern::new(b),
                        s,
                    ));
                    new_state.valuation.add(output_vn, shift_expr.simplify());
                }
            }

            PcodeOperation::IntSignedRightShift { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let s = std::cmp::max(a.size(), b.size());
                    let shift_expr = Value::IntSignedRightShift(value::IntSignedRightShiftExpr(
                        Intern::new(a),
                        Intern::new(b),
                        s,
                    ));
                    new_state.valuation.add(output_vn, shift_expr.simplify());
                }
            }

            PcodeOperation::IntNegate { input, .. } => {
                let a = Value::const_(0);
                let b = Value::from_varnode_or_entry(self, input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, (a - b).simplify());
                }
            }

            PcodeOperation::Int2Comp { input, .. } => {
                let a = Value::from_varnode_or_entry(self, input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_2comp(a).simplify());
                }
            }

            PcodeOperation::Load { input, .. } => {
                let ptr = &input.pointer_location();
                let pv = Value::from_varnode_or_entry(self, ptr);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let load_expr = Value::Load(Load(Intern::new(pv.simplify()), output_vn.size()));
                    if let Some(v) = self.valuation.indirect_writes.get(&load_expr) {
                        new_state.valuation.add(output_vn, v.clone());
                    } else {
                        new_state.valuation.add(output_vn, load_expr);
                    }
                }
            }

            PcodeOperation::IntZExt { input, .. } => {
                let v = Value::from_varnode_or_entry(self, input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let out_size = output_vn.size();
                    new_state
                        .valuation
                        .add(output_vn, Value::zero_extend(v, out_size).simplify());
                }
            }

            PcodeOperation::IntSExt { input, .. } => {
                let v = Value::from_varnode_or_entry(self, input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let out_size = output_vn.size();
                    new_state
                        .valuation
                        .add(output_vn, Value::sign_extend(v, out_size).simplify());
                }
            }

            PcodeOperation::SubPiece { input0, input1, .. } => {
                let v = Value::from_varnode_or_entry(self, input0);
                let byte_offset = input1.offset() as usize;
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let out_size = output_vn.size();
                    new_state.valuation.add(
                        output_vn,
                        Value::extract(v, byte_offset, out_size).simplify(),
                    );
                }
            }

            PcodeOperation::IntEqual { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_equal(a, b).simplify());
                }
            }

            PcodeOperation::IntSignedLess { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_sless(a, b).simplify());
                }
            }

            PcodeOperation::IntLess { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_less(a, b).simplify());
                }
            }

            PcodeOperation::PopCount { input, .. } => {
                let a = Value::from_varnode_or_entry(self, input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::popcount(a).simplify());
                }
            }

            PcodeOperation::IntNotEqual { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_not_equal(a, b).simplify());
                }
            }

            PcodeOperation::IntLessEqual { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_less_equal(a, b).simplify());
                }
            }

            PcodeOperation::IntSignedLessEqual { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_sless_equal(a, b).simplify());
                }
            }

            PcodeOperation::IntCarry { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_carry(a, b).simplify());
                }
            }

            PcodeOperation::IntSignedCarry { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_scarry(a, b).simplify());
                }
            }

            PcodeOperation::IntSignedBorrow { input0, input1, .. } => {
                let a = Value::from_varnode_or_entry(self, input0);
                let b = Value::from_varnode_or_entry(self, input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_sborrow(a, b).simplify());
                }
            }

            // Other operations we don't model produce writes of Top.
            _ => {
                if let Some(GeneralizedVarNode::Direct(vn)) = op.output() {
                    // todo handle indirect
                    new_state.valuation.add(vn, Value::Top);
                }
            }
        }

        // Clear internal-space varnodes on control-flow to non-const destinations (same policy as direct_valuation.rs)
        match op {
            PcodeOperation::Branch { input }
            | PcodeOperation::CBranch { input0: input, .. }
            | PcodeOperation::Fallthrough { input } => {
                if !input.is_const() {
                    // VarNodeMap doesn't provide `retain`; collect keys to remove and remove them.
                    let mut to_remove: Vec<VarNode> = Vec::new();
                    for (vn, _) in new_state.valuation.direct_writes.items() {
                        let keep = self
                            .arch_info
                            .get_space(vn.space_index())
                            .map(|s| s._type != SpaceType::IPTR_CONSTANT)
                            .unwrap_or(false);
                        if !keep {
                            to_remove.push(*vn);
                        }
                    }
                    for k in to_remove {
                        new_state.valuation.direct_writes.remove(k);
                    }
                }
            }
            PcodeOperation::BranchInd { input } | PcodeOperation::CallInd { input } => {
                // Clear IPTR_INTERNAL varnodes except the branch target, which must survive
                // so that strengthen_from_valuation can read it.
                let branch_target = input.pointer_location();
                let mut to_remove: Vec<VarNode> = Vec::new();
                for (vn, _) in new_state.valuation.direct_writes.items() {
                    if vn == branch_target {
                        continue;
                    }
                    let keep = self
                        .arch_info
                        .get_space(vn.space_index())
                        .map(|space| space._type != SpaceType::IPTR_INTERNAL)
                        .unwrap_or(true);
                    if !keep {
                        to_remove.push(*vn);
                    }
                }
                for k in to_remove {
                    new_state.valuation.direct_writes.remove(k);
                }
            }
            PcodeOperation::Call { call_info, .. } => {
                if let Some(a) = call_info.iter().flat_map(|a| a.extrapop).next() {
                    if let Some(stack) = self.arch_info.stack_pointer() {
                        let stack_value = Value::from_varnode_or_entry(self, &stack);
                        let shift_vn = VarNode::new_const(a as u64, stack.size());

                        new_state
                            .valuation
                            .add(stack, stack_value + Value::const_from_varnode(shift_vn));
                    }
                }
            }
            _ => {}
        }

        new_state
    }
}

impl PartialOrd for ValuationState {
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

impl JoinSemiLattice for ValuationState {
    fn join(&mut self, other: &Self) {
        // Merge direct writes
        for (key, other_val) in other.valuation.direct_writes.items() {
            match self.valuation.direct_writes.get_mut(key) {
                Some(my_val) => {
                    if my_val == &Value::Top || other_val == &Value::Top {
                        *my_val = Value::Top;
                    } else if my_val != other_val {
                        match self.merge_behavior {
                            MergeBehavior::Choice => {
                                let combined = Value::choice(my_val.clone(), other_val.clone());
                                *my_val = combined.simplify();
                            }
                            MergeBehavior::Top => {
                                *my_val = Value::Top;
                            }
                        }
                    }
                }
                None => {
                    match self.merge_behavior {
                        MergeBehavior::Choice => {
                            let entry = Value::from_varnode_or_entry(self, key);
                            let choice = Value::choice(entry, other_val.clone());
                            self.valuation.add(*key, choice.simplify());
                        }
                        MergeBehavior::Top => {
                            // If the other state has a direct write that we don't, we have to assume it could be anything.
                            self.valuation.add(*key, Value::Top);
                            continue;
                        }
                    }
                }
            }
        }

        // Merge indirect writes (pointer -> value)
        for (key, other_val) in &other.valuation.indirect_writes {
            match self.valuation.indirect_writes.get_mut(key) {
                Some(my_val) => {
                    if my_val == &Value::Top || other_val == &Value::Top {
                        *my_val = Value::Top;
                    } else if my_val != other_val {
                        match self.merge_behavior {
                            MergeBehavior::Choice => {
                                let combined = Value::choice(my_val.clone(), other_val.clone());
                                *my_val = combined.simplify();
                            }
                            MergeBehavior::Top => {
                                *my_val = Value::Top;
                            }
                        }
                    }
                }
                None => {
                    self.valuation.add(key.clone(), other_val.clone());
                }
            }
        }
    }
}

impl AbstractState for ValuationState {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
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

pub struct ValuationAnalysis {
    arch_info: SleighArchInfo,
    /// Default merge behavior for states produced by this analysis.
    merge_behavior: MergeBehavior,
}

impl ValuationAnalysis {
    /// Create with the default merge behavior (`Or`).
    pub fn new(arch_info: SleighArchInfo, merge_behavior: MergeBehavior) -> Self {
        Self {
            arch_info,
            merge_behavior,
        }
    }
}

impl ConfigurableProgramAnalysis for ValuationAnalysis {
    type State = ValuationState;
    type Reducer<'op> = EmptyResidue<Self::State>;
}

impl IntoState<ValuationAnalysis> for ConcretePcodeAddress {
    fn into_state(
        self,
        c: &ValuationAnalysis,
    ) -> <ValuationAnalysis as ConfigurableProgramAnalysis>::State {
        ValuationState {
            valuation: ValuationSet::new(),
            arch_info: c.arch_info.clone(),
            merge_behavior: c.merge_behavior,
        }
    }
}
