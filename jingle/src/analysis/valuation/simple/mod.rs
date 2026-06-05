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
use std::rc::Rc;

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
        self.valuation
            .direct_writes
            .get(varnode)
            .map(|rc| rc.as_ref())
    }

    pub fn written_locations(&self) -> &VarNodeMap<Rc<Value>> {
        &self.valuation.direct_writes
    }

    pub fn valuation(&self) -> &ValuationSet {
        &self.valuation
    }

    /// Resolve a `VarNode` to its stored value, a `Const` if it is a constant varnode,
    /// or an `Entry` (symbolic unknown) if it has not been written yet.
    pub fn read_vn(&self, vn: &VarNode) -> Rc<Value> {
        if vn.is_const() {
            Rc::new(Value::const_from_varnode(*vn))
        } else if let Some(v) = self.valuation.direct_writes.get(vn) {
            Rc::clone(v)
        } else if let Some((wider_vn, wider_val)) = self
            .valuation
            .direct_writes
            .items()
            .find(|(w, _)| w.covers(vn) && *w != vn)
        {
            // A wider register covers this varnode (e.g. RAX covers EAX).
            // Emit an Extract so simplify() can reduce extract(zext(x, 8), 0, 4) → x.
            let byte_offset = (vn.offset() - wider_vn.offset()) as usize;
            Rc::new(Value::extract(wider_val, byte_offset, vn.size()))
        } else {
            // Assemble the wide register from any known sub-parts. This handles the case
            // where e.g. AH (offset=1) was written but RAX was never explicitly written.
            let sub_parts: Vec<(VarNode, Rc<Value>)> = self
                .valuation
                .direct_writes
                .items()
                .filter(|(w, _)| vn.covers(w) && *w != vn)
                .map(|(w, v)| (*w, Rc::clone(v)))
                .collect();

            if sub_parts.is_empty() {
                Rc::new(Value::entry(*vn))
            } else {
                let mut base: Rc<Value> = Rc::new(Value::entry(*vn));
                for (sub_vn, sub_val) in sub_parts {
                    let byte_offset = (sub_vn.offset() - vn.offset()) as usize;
                    let merged = Value::insert_bytes(Rc::clone(&base), sub_val, byte_offset);
                    base = Value::simplify_shared(&Rc::new(merged));
                }
                base
            }
        }
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
                let val = self.read_vn(input);
                let pv = self.read_vn(ptr);
                let data_size = input.size();
                let loc = Value::Load(Load(
                    Value::simplify_shared(&pv),
                    data_size,
                    output.pointer_space_index() as u8,
                ));
                new_state.valuation.add(loc, val);
            }

            // Copy
            PcodeOperation::Copy { input, .. } => {
                let result: Rc<Value> = if input.is_const() {
                    Rc::new(Value::const_(input.offset() as i64, input.size()))
                } else {
                    self.read_vn(input)
                };
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, result);
                }
            }

            PcodeOperation::IntAdd { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::add_rc(a, b));
                }
            }

            PcodeOperation::IntSub { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::sub_rc(a, b));
                }
            }

            PcodeOperation::IntXor { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::xor_rc(a, b));
                }
            }

            PcodeOperation::IntMult { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::mul_rc(a, b));
                }
            }

            PcodeOperation::IntOr { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::or_rc(a, b));
                }
            }

            PcodeOperation::IntAnd { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::and_rc(a, b));
                }
            }

            PcodeOperation::BoolNegate { input, .. } => {
                let a = self.read_vn(input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::bool_negate(a));
                }
            }

            PcodeOperation::BoolAnd { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::bool_and(a, b));
                }
            }

            PcodeOperation::BoolOr { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::bool_or(a, b));
                }
            }

            PcodeOperation::BoolXor { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::bool_xor(a, b));
                }
            }

            PcodeOperation::IntLeftShift { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let s = std::cmp::max(a.size(), b.size());
                    let shift_expr = Value::IntLeftShift(value::IntLeftShiftExpr(a, b, s));
                    new_state.valuation.add(output_vn, shift_expr);
                }
            }

            PcodeOperation::IntRightShift { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let s = std::cmp::max(a.size(), b.size());
                    let shift_expr = Value::IntRightShift(value::IntRightShiftExpr(a, b, s));
                    new_state.valuation.add(output_vn, shift_expr);
                }
            }

            PcodeOperation::IntSignedRightShift { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let s = std::cmp::max(a.size(), b.size());
                    let shift_expr =
                        Value::IntSignedRightShift(value::IntSignedRightShiftExpr(a, b, s));
                    new_state.valuation.add(output_vn, shift_expr);
                }
            }

            PcodeOperation::IntNegate { input, .. } => {
                let a = Rc::new(Value::const_(0, input.size()));
                let b = self.read_vn(input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::sub_rc(a, b));
                }
            }

            PcodeOperation::Int2Comp { input, .. } => {
                let a = self.read_vn(input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::int_2comp(a));
                }
            }

            PcodeOperation::Load { input, .. } => {
                let ptr = &input.pointer_location();
                let pv = self.read_vn(ptr);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let simplified_ptr = Value::simplify_shared(&pv);
                    let access_vn =
                        VarNode::new(0, output_vn.size() as u32, input.pointer_space_index() as u32);
                    let cached = self
                        .valuation
                        .indirect_writes
                        .get(simplified_ptr.as_ref())
                        .and_then(|inner| inner.get(access_vn).map(Rc::clone));
                    if let Some(v) = cached {
                        new_state.valuation.add(output_vn, v);
                    } else {
                        let load_expr = Value::load(
                            simplified_ptr,
                            output_vn.size(),
                            input.pointer_space_index() as u8,
                        );
                        new_state.valuation.add(output_vn, load_expr);
                    }
                }
            }

            PcodeOperation::IntZExt { input, .. } => {
                let v = self.read_vn(input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let out_size = output_vn.size();
                    new_state
                        .valuation
                        .add(output_vn, Value::zero_extend(v, out_size));
                }
            }

            PcodeOperation::IntSExt { input, .. } => {
                let v = self.read_vn(input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let out_size = output_vn.size();
                    new_state
                        .valuation
                        .add(output_vn, Value::sign_extend(v, out_size));
                }
            }

            PcodeOperation::SubPiece { input0, input1, .. } => {
                let v = self.read_vn(input0);
                let byte_offset = input1.offset() as usize;
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    let out_size = output_vn.size();
                    new_state
                        .valuation
                        .add(output_vn, Value::extract(v, byte_offset, out_size));
                }
            }

            PcodeOperation::IntEqual { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::int_equal(a, b));
                }
            }

            PcodeOperation::IntSignedLess { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::int_sless(a, b));
                }
            }

            PcodeOperation::IntLess { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::int_less(a, b));
                }
            }

            PcodeOperation::PopCount { input, .. } => {
                let a = self.read_vn(input);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::popcount(a));
                }
            }

            PcodeOperation::IntNotEqual { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_not_equal(a, b));
                }
            }

            PcodeOperation::IntLessEqual { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_less_equal(a, b));
                }
            }

            PcodeOperation::IntSignedLessEqual { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state
                        .valuation
                        .add(output_vn, Value::int_sless_equal(a, b));
                }
            }

            PcodeOperation::IntCarry { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::int_carry(a, b));
                }
            }

            PcodeOperation::IntSignedCarry { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::int_scarry(a, b));
                }
            }

            PcodeOperation::IntSignedBorrow { input0, input1, .. } => {
                let a = self.read_vn(input0);
                let b = self.read_vn(input1);
                if let Some(GeneralizedVarNode::Direct(output_vn)) = op.output() {
                    new_state.valuation.add(output_vn, Value::int_sborrow(a, b));
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
            | PcodeOperation::Fallthrough { input }
                if !input.is_const() =>
            {
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
            PcodeOperation::Call { .. } => {}
            PcodeOperation::CallOther {
                output: Some(a), ..
            } => {
                new_state.valuation.add(a, Value::Top);
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
        let self_count: usize = self.valuation.indirect_writes.iter().map(|(_, m)| m.len()).sum();
        let other_count: usize = other
            .valuation
            .indirect_writes
            .iter()
            .map(|(_, m)| m.len())
            .sum();
        if self_count != other_count {
            return None;
        }
        for (ptr, other_inner) in &other.valuation.indirect_writes {
            let my_inner = self.valuation.indirect_writes.get(ptr)?;
            for (vn, other_val) in other_inner.items() {
                match my_inner.get(*vn) {
                    Some(my_val) if my_val == other_val => {}
                    _ => return None,
                }
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
                    if my_val.as_ref() == &Value::Top || other_val.as_ref() == &Value::Top {
                        *my_val = Rc::new(Value::Top);
                    } else if my_val != other_val {
                        match self.merge_behavior {
                            MergeBehavior::Choice => {
                                let combined =
                                    Rc::new(Value::choice(Rc::clone(my_val), Rc::clone(other_val)));
                                *my_val = Value::simplify_shared(&combined);
                            }
                            MergeBehavior::Top => {
                                *my_val = Rc::new(Value::Top);
                            }
                        }
                    }
                }
                None => {
                    match self.merge_behavior {
                        MergeBehavior::Choice => {
                            let entry = self.read_vn(key);
                            let choice = Value::choice(entry, Rc::clone(other_val));
                            self.valuation.add(*key, choice);
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

        // Merge indirect writes (ptr -> VarNodeMap)
        for (ptr, other_inner) in &other.valuation.indirect_writes {
            for (vn, other_val) in other_inner.items() {
                let my_val_opt = self
                    .valuation
                    .indirect_writes
                    .get_mut(ptr)
                    .and_then(|m| m.get_mut(*vn));
                match my_val_opt {
                    Some(my_val) => {
                        if my_val.as_ref() == &Value::Top || other_val.as_ref() == &Value::Top {
                            *my_val = Rc::new(Value::Top);
                        } else if my_val != other_val {
                            match self.merge_behavior {
                                MergeBehavior::Choice => {
                                    let combined = Rc::new(Value::choice(
                                        Rc::clone(my_val),
                                        Rc::clone(other_val),
                                    ));
                                    *my_val = Value::simplify_shared(&combined);
                                }
                                MergeBehavior::Top => {
                                    *my_val = Rc::new(Value::Top);
                                }
                            }
                        }
                    }
                    None => {
                        let load_key =
                            Value::load(ptr.clone(), vn.size(), vn.space_index() as u8);
                        match self.merge_behavior {
                            MergeBehavior::Choice => {
                                let choice =
                                    Value::choice(load_key.clone(), Rc::clone(other_val));
                                self.valuation.add(load_key, choice);
                            }
                            MergeBehavior::Top => {
                                self.valuation.add(load_key, Value::Top);
                            }
                        }
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use jingle_sleigh::{SleighEndianness, SpaceInfo, SpaceType};

    fn test_arch() -> SleighArchInfo {
        SleighArchInfo::new(
            "test:LE:64:default".to_string(),
            std::iter::empty(),
            vec![
                SpaceInfo {
                    name: "const".to_string(),
                    index: 0,
                    index_size_bytes: 8,
                    word_size_bytes: 1,
                    _type: SpaceType::IPTR_CONSTANT,
                    endianness: SleighEndianness::Little,
                },
                SpaceInfo {
                    name: "register".to_string(),
                    index: 1,
                    index_size_bytes: 8,
                    word_size_bytes: 1,
                    _type: SpaceType::IPTR_PROCESSOR,
                    endianness: SleighEndianness::Little,
                },
            ]
            .into_iter(),
            1,
            vec![],
        )
    }

    fn reg(offset: u64, size: usize) -> VarNode {
        VarNode::new(offset, size, 1)
    }

    #[test]
    fn transfer_bool_negate_folds_constant() {
        let state = ValuationState::new(test_arch());
        let out = reg(0x10, 1);
        let op = PcodeOperation::BoolNegate {
            output: out,
            input: VarNode::new_const(0, 1),
        };

        let next = state.transfer_impl(&op);

        assert_eq!(next.get_value(&out), Some(&Value::const_(1, 1)));
    }

    #[test]
    fn transfer_bool_and_uses_boolean_value_node() {
        let state = ValuationState::new(test_arch());
        let out = reg(0x10, 1);
        let left = reg(0x20, 8);
        let right = reg(0x30, 8);
        let op = PcodeOperation::BoolAnd {
            output: out,
            input0: left,
            input1: right,
        };

        let next = state.transfer_impl(&op);
        let value = next.get_value(&out).expect("expected output valuation");
        let expr = value.as_bool_and().expect("expected BoolAnd node");

        assert_eq!(expr.0.as_ref(), &Value::entry(left));
        assert_eq!(expr.1.as_ref(), &Value::entry(right));
    }

    #[test]
    fn transfer_bool_xor_folds_to_bool_negate_when_rhs_true() {
        let mut state = ValuationState::new(test_arch());
        let out = reg(0x10, 1);
        let flag = reg(0x20, 1);
        state.valuation.add(
            flag,
            Value::int_equal(Value::entry(reg(0x30, 8)), Value::entry(reg(0x40, 8))),
        );
        let op = PcodeOperation::BoolXor {
            output: out,
            input0: flag,
            input1: VarNode::new_const(1, 1),
        };

        let next = state.transfer_impl(&op);
        let value = next.get_value(&out).expect("expected output valuation");

        assert_eq!(
            *value,
            Value::int_not_equal(Value::entry(reg(0x30, 8)), Value::entry(reg(0x40, 8))).simplify()
        );
    }

    #[test]
    fn transfer_read_sub_register_after_wider_zext_write() {
        // EAX is written, then RAX = INT_ZEXT EAX (evicts EAX), then ZF = (EAX == 0).
        // ZF must reflect the computed EAX value, not Entry(EAX).
        let mut state = ValuationState::new(test_arch());
        let eax = reg(0x0, 4);
        let rax = reg(0x0, 8);
        let zf = reg(0x100, 1);
        let src = reg(0x200, 4);

        state
            .valuation
            .add(eax, Value::entry(src) + Value::const_(1, 4));

        // RAX = INT_ZEXT EAX — this evicts EAX from direct_writes
        state = state.transfer_impl(&PcodeOperation::IntZExt {
            input: eax,
            output: rax,
        });
        assert!(
            state.get_value(&eax).is_none(),
            "EAX should be evicted after RAX write"
        );

        // ZF = (EAX == 0)
        state = state.transfer_impl(&PcodeOperation::IntEqual {
            output: zf,
            input0: eax,
            input1: VarNode::new_const(0, 4),
        });

        let expected =
            Value::int_equal(Value::entry(src) + Value::const_(1, 4), Value::const_(0, 4))
                .simplify();
        assert_eq!(*state.get_value(&zf).expect("ZF should be set"), expected);
    }

    #[test]
    fn transfer_read_sub_register_nonzero_byte_offset() {
        // Write a 4-byte register at offset 0, then read the upper 2 bytes (offset 2).
        // The result should be extract(wider_val, 2, 2), not Entry(upper).
        let mut state = ValuationState::new(test_arch());
        let wide = reg(0x0, 4);
        let upper = reg(0x2, 2); // bytes [2..4] of wide
        let out = reg(0x100, 1);
        let src = reg(0x200, 4);

        state.valuation.add(wide, Value::entry(src));

        state = state.transfer_impl(&PcodeOperation::IntEqual {
            output: out,
            input0: upper,
            input1: VarNode::new_const(0, 2),
        });

        let expected =
            Value::int_equal(Value::extract(Value::entry(src), 2, 2), Value::const_(0, 2))
                .simplify();
        assert_eq!(*state.get_value(&out).expect("out should be set"), expected);
    }

    #[test]
    fn transfer_read_wide_register_assembled_from_sub_register_at_nonzero_offset() {
        // Simulates: or ah, 0x2 (writes AH at byte offset 1) followed by reading RAX.
        // RAX is never explicitly written; the result must incorporate the AH write.
        let mut state = ValuationState::new(test_arch());
        let rax = reg(0x0, 8);
        let ah = reg(0x1, 1); // byte 1 of RAX
        let out = reg(0x200, 8);
        let ah_val = Value::const_(0x42, 1);

        state.valuation.add(ah, ah_val.clone());

        state = state.transfer_impl(&PcodeOperation::Copy {
            input: rax,
            output: out,
        });

        let result = state.get_value(&out).expect("out must be set");
        assert_ne!(
            *result,
            Value::entry(rax),
            "should not return bare Entry(RAX)"
        );
        let expected = Value::insert_bytes(Value::entry(rax), ah_val, 1).simplify();
        assert_eq!(*result, expected);
    }

    #[test]
    fn join_indirect_write_missing_in_self_top_behavior_becomes_top() {
        // One branch writes RAX to *[addr]:8; the other branch writes nothing there.
        // With MergeBehavior::Top, the merged result should be Top.
        let arch = test_arch();
        let addr_reg = reg(0x1000, 8);
        let rax = reg(0x2000, 8);

        let load_key = Value::load(Value::entry(addr_reg), 8, 1);

        let mut self_state = ValuationState::new_with_behavior(arch.clone(), MergeBehavior::Top);
        let mut other_state = ValuationState::new_with_behavior(arch, MergeBehavior::Top);
        other_state
            .valuation
            .add(load_key.clone(), Value::entry(rax));

        self_state.join(&other_state);

        assert_eq!(
            self_state
                .valuation
                .get(crate::analysis::valuation::simple::valuation::Location::Indirect(
                    load_key
                )),
            Some(&Value::Top),
        );
    }

    #[test]
    fn join_indirect_write_missing_in_self_choice_behavior_becomes_choice() {
        // One branch writes a constant to *[addr]:8; the other branch writes nothing there.
        // With MergeBehavior::Choice, the merged result should be Choice(load_key, written_value).
        let arch = test_arch();
        let addr_reg = reg(0x1000, 8);

        let load_key = Value::load(Value::entry(addr_reg), 8, 1);
        let written = Value::const_(0x42, 8);

        let mut self_state = ValuationState::new_with_behavior(arch.clone(), MergeBehavior::Choice);
        let mut other_state = ValuationState::new_with_behavior(arch, MergeBehavior::Choice);
        other_state.valuation.add(load_key.clone(), written.clone());

        self_state.join(&other_state);

        let result = self_state
            .valuation
            .get(crate::analysis::valuation::simple::valuation::Location::Indirect(
                load_key.clone(),
            ))
            .expect("should have an entry after join");
        let choice = result.as_choice().expect("expected a Choice node");
        assert!(
            (choice.0.as_ref() == &load_key && choice.1.as_ref() == &written)
                || (choice.1.as_ref() == &load_key && choice.0.as_ref() == &written),
            "expected Choice(load_key, written) in any order, got {result:?}",
        );
    }
}
