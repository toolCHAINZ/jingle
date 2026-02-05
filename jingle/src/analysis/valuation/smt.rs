use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::residue::EmptyResidue;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::varnode_map::VarNodeMap;
use crate::display::JingleDisplay;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{GeneralizedVarNode, PcodeOperation, SleighArchInfo, SpaceType, VarNode};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

// Z3 bitvector support (assume updated API without explicit Context lifetimes)
// Import the `Ast` trait so we can call `simplify()` and `ite()` on AST nodes.
use z3::ast::{Ast, BV};

/// SMT-backed valuation for varnodes:
/// - `Val(BV)` stores an actual bitvector expression
/// - `Load(ptr)` stores the pointer expression used for the load (not the loaded value)
/// - `Top` represents an unknown / unconstrained value
#[derive(Clone, Debug)]
pub enum SmtVal {
    Val(BV),
    Load(Rc<SmtVal>),
    Or(Rc<(SmtVal, SmtVal)>),
    Top,
}

impl PartialEq for SmtVal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SmtVal::Top, SmtVal::Top) => true,
            (SmtVal::Val(a), SmtVal::Val(b)) => a.to_string() == b.to_string(),
            (SmtVal::Load(pa), SmtVal::Load(pb)) => pa == pb,
            (SmtVal::Or(a), SmtVal::Or(b)) => a.as_ref() == b.as_ref(),
            _ => false,
        }
    }
}
impl Eq for SmtVal {}

impl Hash for SmtVal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            SmtVal::Top => {
                "Top".hash(state);
            }
            SmtVal::Val(bv) => {
                bv.to_string().hash(state);
            }
            SmtVal::Load(inner) => {
                "Load".hash(state);
                inner.hash(state);
            }
            SmtVal::Or(pair) => {
                "Or".hash(state);
                pair.as_ref().0.hash(state);
                pair.as_ref().1.hash(state);
            }
        }
    }
}

impl JingleDisplay for SmtVal {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        match self {
            SmtVal::Top => write!(f, "⊤"),
            SmtVal::Val(v) => write!(f, "{}", v),
            SmtVal::Load(ptr) => write!(f, "Load({})", ptr.display(info)),
            SmtVal::Or(pair) => write!(
                f,
                "({}||{})",
                pair.as_ref().0.display(info),
                pair.as_ref().1.display(info)
            ),
        }
    }
}

fn simplify_smtval(v: SmtVal) -> SmtVal {
    match v {
        SmtVal::Val(bv) => {
            // simplify the BV AST using Z3's simplify
            let s = bv.simplify();
            SmtVal::Val(s)
        }
        SmtVal::Load(ptr) => {
            // recursively simplify the inner pointer expression
            let inner = simplify_smtval((*ptr).clone());
            SmtVal::Load(Rc::new(inner))
        }
        SmtVal::Or(pair) => {
            // simplify both sides of the Or
            let left_s = simplify_smtval(pair.0.clone());
            let right_s = simplify_smtval(pair.1.clone());

            // Collect flattened non-Or parts from nested Ors.
            fn collect_parts(e: SmtVal, out: &mut Vec<SmtVal>) {
                match e {
                    SmtVal::Or(p) => {
                        // p.0 and p.1 are owned SmtVal; recurse into them
                        let l = p.0.clone();
                        let r = p.1.clone();
                        collect_parts(l, out);
                        collect_parts(r, out);
                    }
                    other => out.push(other),
                }
            }

            let mut parts: Vec<SmtVal> = Vec::new();
            collect_parts(left_s, &mut parts);
            collect_parts(right_s, &mut parts);

            // Deduplicate while preserving order.
            let mut uniq: Vec<SmtVal> = Vec::new();
            'outer: for p in parts {
                for q in &uniq {
                    if p == *q {
                        continue 'outer;
                    }
                }
                uniq.push(p);
            }

            // Collapse according to the number of unique parts.
            match uniq.len() {
                0 => SmtVal::Top, // defensive: shouldn't happen, but be conservative
                1 => uniq.into_iter().next().unwrap(),
                _ => {
                    // Fold into a left-associative chain of Ors.
                    let mut iter = uniq.into_iter();
                    let first = iter.next().unwrap();
                    let mut acc = first;
                    for item in iter {
                        acc = SmtVal::Or(Rc::new((acc, item)));
                    }
                    acc
                }
            }
        }
        SmtVal::Top => SmtVal::Top,
    }
}

// Reuse the `MergeBehavior` defined by the simple valuation module so both analyses share the same enum.
use crate::analysis::valuation::MergeBehavior;

/// State for the SMT-backed direct valuation CPA.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SmtValuationState {
    written_locations: VarNodeMap<SmtVal>,
    /// Cache of entry (initial) bitvector variables for varnodes that haven't been written to.
    entry_cache: VarNodeMap<BV>,
    /// Cache of the deterministic names used for entry variables so we always create a
    /// `BV` with the same name for the same `VarNode` (prevents duplicate `fresh_const` variants).
    name_cache: VarNodeMap<String>,
    arch_info: SleighArchInfo,
    merge_behavior: MergeBehavior,
    /// Small (u16) identifier used to disambiguate entry variables coming from different
    /// analysis invocations / starting states. This value is preserved across `clone`.
    id: u16,
    // Use Z3's deterministic-named constants to avoid duplicate hints becoming different fresh symbols.
}

impl SmtValuationState {
    // No local fresh_name helper: we use Z3's `fresh_const` APIs to obtain unique names.

    /// Return an `SmtVal` for a varnode: constants -> `Val(BV)`, written locations -> stored valuation,
    /// otherwise create/reuse a named entry `BV`.
    fn get_valuation_or_entry(&mut self, vn: &VarNode) -> SmtVal {
        // Constant literal -> concrete BV
        if vn.space_index == VarNode::CONST_SPACE_INDEX {
            let bits = (vn.size * 8) as u32;
            return SmtVal::Val(BV::from_u64(vn.offset, bits));
        }

        // If we've written to this location, return stored valuation
        if let Some(v) = self.written_locations.get(vn) {
            return v.clone();
        }

        // Otherwise, return (or create) a named BV representing the entry value
        if let Some(cached) = self.entry_cache.get(vn) {
            return SmtVal::Val(cached.clone());
        }

        let display_str = format!("{}", vn.display(&self.arch_info));
        let sanitized = display_str.replace(' ', "_");
        // Include the state's unique id in the entry variable name to avoid collisions
        // between different analysis invocations / starting states.
        let base = format!("entry_{}_{}", sanitized, self.id);
        let bits = (vn.size * 8) as u32;

        // If we already chose a deterministic name for this VarNode, reuse it and create a BV
        // with that name (so we consistently refer to the same symbol across the analysis).
        if let Some(name) = self.name_cache.get(vn) {
            let bv = BV::new_const(name.as_str(), bits);
            self.entry_cache.insert(vn.clone(), bv.clone());
            return SmtVal::Val(bv);
        }

        // Otherwise, pick a deterministic name (based on the varnode display) and remember it.
        // Use `new_const` with this deterministic name so future requests for the same varnode
        // produce a BV with the same printed name (avoids duplicate hints getting different `!n` suffixes).
        let chosen_name = base;
        self.name_cache.insert(vn.clone(), chosen_name.clone());
        let bv = BV::new_const(chosen_name.as_str(), bits);
        self.entry_cache.insert(vn.clone(), bv.clone());
        SmtVal::Val(bv)
    }

    pub fn new(arch_info: SleighArchInfo) -> Self {
        // Derive a compact u16 id from the arch info so states get an identifier even when
        // created via this constructor. This id will persist through clone.
        let mut hasher = DefaultHasher::new();
        arch_info.hash(&mut hasher);
        let id = (hasher.finish() & 0xffff) as u16;

        Self {
            written_locations: VarNodeMap::new(),
            entry_cache: VarNodeMap::new(),
            name_cache: VarNodeMap::new(),
            arch_info,
            merge_behavior: MergeBehavior::Or,
            id,
        }
    }

    pub fn new_with_behavior(arch_info: SleighArchInfo, merge_behavior: MergeBehavior) -> Self {
        // Derive a compact u16 id from the arch info so states get an identifier even when
        // created via this constructor. This id will persist through clone.
        let mut hasher = DefaultHasher::new();
        arch_info.hash(&mut hasher);
        let id = (hasher.finish() & 0xffff) as u16;

        Self {
            written_locations: VarNodeMap::new(),
            entry_cache: VarNodeMap::new(),
            name_cache: VarNodeMap::new(),
            arch_info,
            merge_behavior,
            id,
        }
    }

    pub fn get_value(&self, varnode: &VarNode) -> Option<&SmtVal> {
        self.written_locations.get(varnode)
    }

    pub fn written_locations(&self) -> &VarNodeMap<SmtVal> {
        &self.written_locations
    }

    /// Transfer function: build SMT BV valuations for pcode operations.
    /// Loads are represented as `SmtVal::Load(pointer_expr)` rather than the loaded value.
    fn transfer_impl(&self, op: &PcodeOperation) -> Self {
        // Clone self to build a new state (functional update).
        let mut new_state = self.clone();

        if let Some(output) = op.output() {
            match output {
                GeneralizedVarNode::Direct(output_vn) => {
                    let result_val = match op {
                        // Copy
                        PcodeOperation::Copy { input, .. } => {
                            if input.space_index == VarNode::CONST_SPACE_INDEX {
                                SmtVal::Val(BV::from_u64(input.offset, (input.size * 8) as u32))
                            } else {
                                new_state.get_valuation_or_entry(input)
                            }
                        }

                        // Arithmetic
                        PcodeOperation::IntAdd { input0, input1, .. } => {
                            let a = new_state.get_valuation_or_entry(input0);
                            let b = new_state.get_valuation_or_entry(input1);
                            match (a, b) {
                                (SmtVal::Val(a), SmtVal::Val(b)) => SmtVal::Val(a.bvadd(&b)),
                                _ => SmtVal::Top,
                            }
                        }

                        PcodeOperation::IntSub { input0, input1, .. } => {
                            let a = new_state.get_valuation_or_entry(input0);
                            let b = new_state.get_valuation_or_entry(input1);
                            match (a, b) {
                                (SmtVal::Val(a), SmtVal::Val(b)) => SmtVal::Val(a.bvsub(&b)),
                                _ => SmtVal::Top,
                            }
                        }

                        PcodeOperation::IntMult { input0, input1, .. } => {
                            let a = new_state.get_valuation_or_entry(input0);
                            let b = new_state.get_valuation_or_entry(input1);
                            match (a, b) {
                                (SmtVal::Val(a), SmtVal::Val(b)) => SmtVal::Val(a.bvmul(&b)),
                                _ => SmtVal::Top,
                            }
                        }

                        // Bitwise operations
                        PcodeOperation::IntAnd { input0, input1, .. }
                        | PcodeOperation::BoolAnd { input0, input1, .. } => {
                            let a = new_state.get_valuation_or_entry(input0);
                            let b = new_state.get_valuation_or_entry(input1);
                            match (a, b) {
                                (SmtVal::Val(a), SmtVal::Val(b)) => SmtVal::Val(a.bvand(&b)),
                                _ => SmtVal::Top,
                            }
                        }

                        PcodeOperation::IntXor { input0, input1, .. }
                        | PcodeOperation::BoolXor { input0, input1, .. } => {
                            let a = new_state.get_valuation_or_entry(input0);
                            let b = new_state.get_valuation_or_entry(input1);
                            match (a, b) {
                                (SmtVal::Val(a), SmtVal::Val(b)) => SmtVal::Val(a.bvxor(&b)),
                                _ => SmtVal::Top,
                            }
                        }

                        PcodeOperation::IntOr { input0, input1, .. }
                        | PcodeOperation::BoolOr { input0, input1, .. } => {
                            let a = new_state.get_valuation_or_entry(input0);
                            let b = new_state.get_valuation_or_entry(input1);
                            match (a, b) {
                                (SmtVal::Val(a), SmtVal::Val(b)) => SmtVal::Val(a.bvor(&b)),
                                _ => SmtVal::Top,
                            }
                        }

                        PcodeOperation::IntLeftShift { input0, input1, .. }
                        | PcodeOperation::IntRightShift { input0, input1, .. }
                        | PcodeOperation::IntSignedRightShift { input0, input1, .. } => {
                            // Approximate shifts as Add of operands (conservative)
                            let a = new_state.get_valuation_or_entry(input0);
                            let b = new_state.get_valuation_or_entry(input1);
                            match (a, b) {
                                (SmtVal::Val(a), SmtVal::Val(b)) => SmtVal::Val(a.bvadd(&b)),
                                _ => SmtVal::Top,
                            }
                        }

                        PcodeOperation::IntNegate { input, .. } => {
                            let a = new_state.get_valuation_or_entry(input);
                            match a {
                                SmtVal::Val(a) => {
                                    let bits = a.get_size();
                                    let zero = BV::from_u64(0, bits);
                                    SmtVal::Val(zero.bvsub(&a))
                                }
                                _ => SmtVal::Top,
                            }
                        }

                        PcodeOperation::Int2Comp { input, .. } => {
                            let a = new_state.get_valuation_or_entry(input);
                            match a {
                                SmtVal::Val(a) => SmtVal::Val(a.bvnot()),
                                _ => SmtVal::Top,
                            }
                        }

                        // Load - store the pointer expression (do NOT load the value here)
                        PcodeOperation::Load { input, .. } => {
                            let ptr = &input.pointer_location;
                            let pv = if ptr.space_index == VarNode::CONST_SPACE_INDEX {
                                SmtVal::Val(BV::from_u64(ptr.offset, (ptr.size * 8) as u32))
                            } else {
                                new_state.get_valuation_or_entry(ptr)
                            };
                            SmtVal::Load(Rc::new(pv))
                        }

                        // Casts/extensions - preserve symbolic value via BV ops
                        PcodeOperation::IntSExt { input, .. }
                        | PcodeOperation::IntZExt { input, .. } => {
                            let a = new_state.get_valuation_or_entry(input);
                            match a {
                                SmtVal::Val(a) => {
                                    let in_bits = a.get_size();
                                    let out_bits = (output_vn.size * 8) as u32;
                                    if out_bits >= in_bits {
                                        let ext = out_bits - in_bits;
                                        if matches!(op, PcodeOperation::IntSExt { .. }) {
                                            SmtVal::Val(a.sign_ext(ext))
                                        } else {
                                            SmtVal::Val(a.zero_ext(ext))
                                        }
                                    } else {
                                        // Truncate by extracting low bits
                                        SmtVal::Val(a.extract(out_bits - 1, 0))
                                    }
                                }
                                _ => SmtVal::Top,
                            }
                        }

                        // Default: be conservative and mark as Top
                        _ => SmtVal::Top,
                    };

                    // Insert the computed (or Top) value into written_locations;
                    // simplify the SMT AST first to keep expressions reduced.
                    let simplified = simplify_smtval(result_val);
                    new_state
                        .written_locations
                        .insert(output_vn.clone(), simplified);
                }

                GeneralizedVarNode::Indirect(_) => {
                    // Indirect writes are not tracked by this CPA.
                }
            }
        }

        // Clear internal-space varnodes on control-flow to non-const destinations (same policy)
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

impl Hash for SmtValuationState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // `VarNodeMap` stores keys in sorted order; iterate deterministically.
        for (vn, val) in self.written_locations.iter() {
            vn.hash(state);
            val.hash(state);
        }
        // include merge behavior and arch info in the hash
        self.merge_behavior.hash(state);
        self.arch_info.hash(state);
    }
}

impl PartialOrd for SmtVal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

impl JoinSemiLattice for SmtVal {
    fn join(&mut self, _other: &Self) {}
}

impl PartialOrd for SmtValuationState {
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

impl JoinSemiLattice for SmtValuationState {
    fn join(&mut self, other: &Self) {
        // For each varnode present in `other`:
        // - if present in self with same valuation -> keep
        // - if present in self with different valuation -> combine according to merge_behavior
        // - if absent in self -> clone from other
        for (key, other_val) in other.written_locations.iter() {
            match self.written_locations.get_mut(key) {
                Some(my_val) => {
                    if my_val == &SmtVal::Top || other_val == &SmtVal::Top {
                        *my_val = SmtVal::Top;
                    } else if my_val != other_val {
                        match self.merge_behavior {
                            MergeBehavior::Or => {
                                // Prefer to create an ite selector when both are `Val`.
                                // If both are `Load(...)`, merge inner pointer expressions into a
                                // single `Load(Or(...))` so we don't create an `Or(Load(...), Load(...))`
                                // which later simplifies less effectively.
                                let combined = {
                                    // `my_val` is `&mut SmtVal`, `other_val` is `&SmtVal`.
                                    // Match on their referenced forms to avoid moving out of borrows.
                                    match (&*my_val, other_val) {
                                        (SmtVal::Load(a_rc), SmtVal::Load(b_rc)) => {
                                            // a_rc and b_rc are &Rc<SmtVal>; obtain owned inner SmtVal clones
                                            let a_inner = (*a_rc).as_ref().clone();
                                            let b_inner = (*b_rc).as_ref().clone();
                                            let inner_or = SmtVal::Or(Rc::new((a_inner, b_inner)));
                                            SmtVal::Load(Rc::new(inner_or))
                                        }
                                        _ => {
                                            // Fallback: create a symbolic Or of the two full values.
                                            // Clone the owned values rather than trying to move them.
                                            SmtVal::Or(Rc::new((
                                                (*my_val).clone(),
                                                other_val.clone(),
                                            )))
                                        }
                                    }
                                };
                                *my_val = simplify_smtval(combined);
                            }
                            MergeBehavior::Top => {
                                *my_val = SmtVal::Top;
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

impl AbstractState for SmtValuationState {
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

impl Display for SmtValuationState {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        let hash_value = hasher.finish();
        write!(f, "Hash({:016x})", hash_value)
    }
}

impl JingleDisplay for SmtValuationState {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        // Render the written locations in a concise form using the Sleigh arch display context.
        write!(f, "SmtValuationState {{")?;
        let mut first = true;
        for (vn, val) in self.written_locations.iter() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            // Use the JingleDisplay implementations for VarNode and SmtVal
            write!(f, "{} = {}", vn.display(info), val.display(info))?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

pub struct SmtValuationAnalysis {
    arch_info: SleighArchInfo,
    /// Default merge behavior for states produced by this analysis.
    merge_behavior: MergeBehavior,
}

impl SmtValuationAnalysis {
    /// Create with the default merge behavior (`Or`).
    pub fn new(arch_info: SleighArchInfo, merge_behavior: MergeBehavior) -> Self {
        Self {
            arch_info,
            merge_behavior,
        }
    }
}

impl ConfigurableProgramAnalysis for SmtValuationAnalysis {
    type State = SmtValuationState;
    type Reducer<'op> = EmptyResidue<Self::State>;
}

impl IntoState<SmtValuationAnalysis> for ConcretePcodeAddress {
    fn into_state(
        self,
        c: &SmtValuationAnalysis,
    ) -> <SmtValuationAnalysis as ConfigurableProgramAnalysis>::State {
        // Compute a compact u16 id derived from the concrete address and analysis arch info.
        // This gives each starting state a small unique identifier that will be preserved when cloned.
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        c.arch_info.hash(&mut hasher);
        let id = (hasher.finish() & 0xffff) as u16;

        SmtValuationState {
            written_locations: VarNodeMap::new(),
            entry_cache: VarNodeMap::new(),
            name_cache: VarNodeMap::new(),
            arch_info: c.arch_info.clone(),
            merge_behavior: c.merge_behavior,
            id,
        }
    }
}
