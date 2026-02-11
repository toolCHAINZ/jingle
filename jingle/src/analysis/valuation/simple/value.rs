use crate::{
    analysis::{cpa::lattice::JoinSemiLattice, valuation::SimpleValuationState},
    display::JingleDisplay,
};
use internment::Intern;
use jingle_sleigh::{SleighArchInfo, VarNode};
use std::{
    cmp::Ordering,
    fmt::Formatter,
    ops::{Add, Mul, Sub},
};

trait Simplify {
    fn simplify(&self) -> SimpleValue;
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Entry(pub Intern<VarNode>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct MulExpr(pub Intern<SimpleValue>, pub Intern<SimpleValue>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AddExpr(pub Intern<SimpleValue>, pub Intern<SimpleValue>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SubExpr(pub Intern<SimpleValue>, pub Intern<SimpleValue>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Or(pub Intern<SimpleValue>, pub Intern<SimpleValue>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Load(pub Intern<SimpleValue>);

/// Symbolic valuation built from varnodes and constants.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SimpleValue {
    Entry(Entry),
    Const(i64),

    // Binary operators now use interned children (via `internment`) rather than Arc'd tuples.
    Mul(MulExpr),
    Add(AddExpr),
    Sub(SubExpr),

    Or(Or),
    Load(Load),
    Top,
}

impl SimpleValue {
    /// Extract constant value if this is a Const variant
    pub fn as_const(&self) -> Option<i64> {
        match self {
            SimpleValue::Const(val) => Some(*val),
            _ => None,
        }
    }

    // --- Convenience constructors -------------------------------------------------

    /// Construct an `Entry(...)` from a `VarNode`.
    pub fn entry(vn: VarNode) -> Self {
        SimpleValue::Entry(Entry(Intern::new(vn)))
    }

    /// Construct a `Const(...)`.
    pub fn const_(v: i64) -> Self {
        SimpleValue::Const(v)
    }

    /// Construct an `Or(...)` node from two children.
    pub fn or(left: SimpleValue, right: SimpleValue) -> Self {
        SimpleValue::Or(Or(Intern::new(left), Intern::new(right)))
    }

    /// Construct a `Load(...)` node from a child.
    pub fn load(child: SimpleValue) -> Self {
        SimpleValue::Load(Load(Intern::new(child)))
    }

    // Keep the older helpers (used by some simplifications) for parity:

    /// Create a constant SimpleValue with the given value and size.
    /// Size is currently not used in this representation, but kept for parity with the
    /// previous implementation which used sizes when constructing sized constants.
    fn make_const(value: i64, _size: usize) -> Self {
        SimpleValue::Const(value)
    }

    /// Helper to pick a reasonable size for a new constant when folding results.
    /// Prefer sizes found on Entry varnodes; fall back to 8 bytes (64-bit).
    fn derive_size_from(val: &SimpleValue) -> usize {
        match val {
            SimpleValue::Entry(vn) => vn.0.as_ref().size,
            _ => 8,
        }
    }

    /// Normalize commutative operands so that constants (if present) are on the right.
    /// Returns (left, right) possibly swapped.
    fn normalize_commutative(left: SimpleValue, right: SimpleValue) -> (SimpleValue, SimpleValue) {
        let left_is_const = left.as_const().is_some();
        let right_is_const = right.as_const().is_some();

        // If left is const and right is not, swap them so constant is on right.
        if left_is_const && !right_is_const {
            (right, left)
        } else {
            (left, right)
        }
    }

    /// Normalize Or operands so that the canonical form has a non-Or on the left
    /// and an Or on the right when one operand is an Or. This makes simplifications
    /// like `Or(Or(a,b), c)` and `Or(c, Or(a,b))` handled uniformly.
    fn normalize_or(left: SimpleValue, right: SimpleValue) -> (SimpleValue, SimpleValue) {
        let left_is_or = matches!(left, SimpleValue::Or(_));
        let right_is_or = matches!(right, SimpleValue::Or(_));

        // If left is an Or and right is not, swap so the Or is on the right.
        if left_is_or && !right_is_or {
            (right, left)
        } else {
            (left, right)
        }
    }

    /// Provide a coarse rank for variants so we can produce deterministic ordering
    /// among different kinds of children when canonicalizing binary commutative nodes.
    fn variant_rank(v: &SimpleValue) -> u8 {
        match v {
            SimpleValue::Const(_) => 0,
            SimpleValue::Entry(_) => 1,
            SimpleValue::Mul(_) => 2,
            SimpleValue::Add(_) => 3,
            SimpleValue::Sub(_) => 4,
            SimpleValue::Or(_) => 5,
            SimpleValue::Load(_) => 6,
            SimpleValue::Top => 7,
        }
    }
}

impl Simplify for SimpleValue {
    fn simplify(&self) -> SimpleValue {
        match self {
            SimpleValue::Mul(expr) => expr.simplify(),
            SimpleValue::Add(expr) => expr.simplify(),
            SimpleValue::Sub(expr) => expr.simplify(),
            SimpleValue::Or(expr) => expr.simplify(),
            SimpleValue::Load(expr) => expr.simplify(),
            SimpleValue::Entry(_) | SimpleValue::Const(_) | SimpleValue::Top => self.clone(),
        }
    }
}

impl Mul for SimpleValue {
    type Output = SimpleValue;

    fn mul(self, rhs: Self) -> Self::Output {
        SimpleValue::Mul(MulExpr(Intern::new(self), Intern::new(rhs)))
    }
}

impl Add for SimpleValue {
    type Output = SimpleValue;

    fn add(self, rhs: Self) -> Self::Output {
        SimpleValue::Add(AddExpr(Intern::new(self), Intern::new(rhs)))
    }
}

impl Sub for SimpleValue {
    type Output = SimpleValue;

    fn sub(self, rhs: Self) -> Self::Output {
        SimpleValue::Sub(SubExpr(Intern::new(self), Intern::new(rhs)))
    }
}

impl SimpleValue {
    /// Inherent simplify method so callers don't need the `Simplify` trait in scope.
    /// This delegates to the same per-variant simplifiers that the `Simplify`
    /// implementations provide for the individual AST node structs.
    pub fn simplify(&self) -> SimpleValue {
        Simplify::simplify(self)
    }
}

impl Simplify for AddExpr {
    fn simplify(&self) -> SimpleValue {
        let a_intern = self.0;
        let b_intern = self.1;

        // simplify children first
        let a_s = a_intern.as_ref().simplify();
        let b_s = b_intern.as_ref().simplify();

        // if any child is Top, the result is Top
        if matches!(a_s, SimpleValue::Top) || matches!(b_s, SimpleValue::Top) {
            return SimpleValue::Top;
        }

        // both const -> fold
        if let (SimpleValue::Const(a), SimpleValue::Const(b)) = (&a_s, &b_s) {
            let res = a.wrapping_add(*b);
            return SimpleValue::Const(res);
        }

        // normalization: ensure constants are on the right
        let (left, right) = SimpleValue::normalize_commutative(a_s, b_s);

        // expr + 0 -> expr
        // expr + (- |a|) -> expr - a
        match right.as_const() {
            Some(0) => {
                return left;
            }
            Some(a) => {
                if a < 0 {
                    let sub = SubExpr(
                        Intern::new(left.clone()),
                        Intern::new(SimpleValue::Const(-a)),
                    )
                    .simplify();
                    return sub;
                }
            }
            _ => {}
        }

        if right.as_const() == Some(0) {
            return left;
        }

        // ((expr + #a) + #b) -> (expr + #(a + b))
        if let SimpleValue::Add(AddExpr(left_inner_left, left_inner_right)) = &left {
            if let SimpleValue::Const(inner_right_const) = left_inner_right.as_ref() {
                if let SimpleValue::Const(right_const) = &right {
                    let res = inner_right_const.wrapping_add(*right_const);
                    let new_const = SimpleValue::Const(res);
                    return AddExpr(*left_inner_left, Intern::new(new_const)).simplify();
                }
            }
        }

        // ((expr - #a) + #b) -> (expr - #(a - b))
        if let SimpleValue::Sub(SubExpr(expr, a)) = &left {
            if let SimpleValue::Const(a_const) = a.as_ref() {
                if let SimpleValue::Const(b) = &right {
                    let res = a_const.wrapping_sub(*b);
                    let new_const = SimpleValue::Const(res);
                    return SubExpr(*expr, Intern::new(new_const)).simplify();
                }
            }
        }

        // default: rebuild with simplified children
        SimpleValue::Add(AddExpr(Intern::new(left), Intern::new(right)))
    }
}

impl Simplify for SubExpr {
    fn simplify(&self) -> SimpleValue {
        let a_intern = self.0;
        let b_intern = self.1;

        let a_s = a_intern.as_ref().simplify();
        let b_s = b_intern.as_ref().simplify();

        if matches!(a_s, SimpleValue::Top) || matches!(b_s, SimpleValue::Top) {
            return SimpleValue::Top;
        }

        // both const -> fold
        if let (SimpleValue::Const(left), SimpleValue::Const(right)) = (&a_s, &b_s) {
            let res = left.wrapping_sub(*right);
            return SimpleValue::Const(res);
        }

        // normalization: ensure constants are on the right
        let (left, right) = SimpleValue::normalize_commutative(a_s, b_s);

        // expr - 0 -> expr
        // expr - (- |a|) -> expr + a
        match right.as_const() {
            Some(0) => {
                return left;
            }
            Some(a) => {
                if a < 0 {
                    let add = AddExpr(
                        Intern::new(left.clone()),
                        Intern::new(SimpleValue::Const(-a)),
                    )
                    .simplify();
                    return add;
                }
            }
            _ => {}
        }

        // x - x -> 0
        if left == right {
            let size = SimpleValue::derive_size_from(&left);
            return SimpleValue::make_const(0, size);
        }

        // ((expr + #a) - #b) -> (expr + #(a - b))
        if let SimpleValue::Add(AddExpr(expr, a)) = &left {
            if let SimpleValue::Const(a) = a.as_ref() {
                if let SimpleValue::Const(b) = &right {
                    let res = a.wrapping_sub(*b);
                    let new_const = SimpleValue::Const(res);
                    return AddExpr(*expr, Intern::new(new_const)).simplify();
                }
            }
        }

        // ((expr - #a) - #b) -> (expr - #(a + b))
        if let SimpleValue::Sub(SubExpr(expr, a)) = &left {
            if let SimpleValue::Const(a) = a.as_ref() {
                if let SimpleValue::Const(b) = &right {
                    let res = a.wrapping_add(*b);
                    let new_const = SimpleValue::Const(res);
                    return SubExpr(*expr, Intern::new(new_const)).simplify();
                }
            }
        }

        SimpleValue::Sub(SubExpr(Intern::new(left), Intern::new(right)))
    }
}

impl Simplify for MulExpr {
    fn simplify(&self) -> SimpleValue {
        let a_intern = self.0;
        let b_intern = self.1;

        let a_s = a_intern.as_ref().simplify();
        let b_s = b_intern.as_ref().simplify();

        if matches!(a_s, SimpleValue::Top) || matches!(b_s, SimpleValue::Top) {
            return SimpleValue::Top;
        }

        // normalization: prefer constant on the right
        let (left, right) = SimpleValue::normalize_commutative(a_s, b_s);

        // both const -> fold
        if let (SimpleValue::Const(a_vn), SimpleValue::Const(b_vn)) = (&left, &right) {
            let res = a_vn.wrapping_mul(*b_vn);
            return SimpleValue::Const(res);
        }

        // expr * 1 -> expr
        if right.as_const() == Some(1) {
            return left;
        }

        // expr * 0 -> 0
        if right.as_const() == Some(0) {
            let size = SimpleValue::derive_size_from(&left);
            return SimpleValue::make_const(0, size);
        }

        SimpleValue::Mul(MulExpr(Intern::new(left), Intern::new(right)))
    }
}

impl Simplify for Or {
    fn simplify(&self) -> SimpleValue {
        let a_intern = self.0;
        let b_intern = self.1;

        // simplify children first
        let a_s = a_intern.as_ref().simplify();
        let b_s = b_intern.as_ref().simplify();

        // if either child is Top, the result is Top
        if matches!(a_s, SimpleValue::Top) || matches!(b_s, SimpleValue::Top) {
            return SimpleValue::Top;
        }

        // normalize so that if one side is an Or and the other is not, the Or is on the right
        // (canonical shape: non-Or on left, Or on right)
        let (mut left, mut right) = SimpleValue::normalize_or(a_s, b_s);

        // If both sides are non-Or, enforce deterministic ordering by variant rank.
        if !matches!(left, SimpleValue::Or(_))
            && !matches!(right, SimpleValue::Or(_))
            && SimpleValue::variant_rank(&left) > SimpleValue::variant_rank(&right)
        {
            std::mem::swap(&mut left, &mut right);
        }

        // identical children => just return one
        if left == right {
            return left;
        }

        // Collapse nested duplicates: Or(a, Or(a, b)) -> Or(a, b)
        if let SimpleValue::Or(Or(inner_a, inner_b)) = &right {
            if inner_a.as_ref() == &left {
                return SimpleValue::Or(Or(Intern::new(left.clone()), *inner_b)).simplify();
            }
            if inner_b.as_ref() == &left {
                return SimpleValue::Or(Or(Intern::new(left.clone()), *inner_a)).simplify();
            }
        }

        // Factor common child between two Ors:
        // Or(Or(a,b), Or(a,c)) -> Or(a, Or(b,c)) and symmetric variants.
        if let (SimpleValue::Or(Or(l1, l2)), SimpleValue::Or(Or(r1, r2))) = (&left, &right) {
            // check all combinations for equal common child
            if l1.as_ref() == r1.as_ref() {
                let inner = SimpleValue::Or(Or(*l2, *r2)).simplify();
                return SimpleValue::Or(Or(Intern::new(l1.as_ref().clone()), Intern::new(inner)))
                    .simplify();
            }
            if l1.as_ref() == r2.as_ref() {
                let inner = SimpleValue::Or(Or(*l2, *r1)).simplify();
                return SimpleValue::Or(Or(Intern::new(l1.as_ref().clone()), Intern::new(inner)))
                    .simplify();
            }
            if l2.as_ref() == r1.as_ref() {
                let inner = SimpleValue::Or(Or(*l1, *r2)).simplify();
                return SimpleValue::Or(Or(Intern::new(l2.as_ref().clone()), Intern::new(inner)))
                    .simplify();
            }
            if l2.as_ref() == r2.as_ref() {
                let inner = SimpleValue::Or(Or(*l1, *r1)).simplify();
                return SimpleValue::Or(Or(Intern::new(l2.as_ref().clone()), Intern::new(inner)))
                    .simplify();
            }
        }

        // default: rebuild with simplified children
        SimpleValue::Or(Or(Intern::new(left), Intern::new(right)))
    }
}

impl Simplify for Load {
    fn simplify(&self) -> SimpleValue {
        let a_intern = self.0;
        let a_s = a_intern.as_ref().simplify();

        if matches!(a_s, SimpleValue::Top) {
            return SimpleValue::Top;
        }

        SimpleValue::Load(Load(Intern::new(a_s)))
    }
}

impl JingleDisplay for SimpleValue {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        match self {
            SimpleValue::Entry(Entry(vn)) => write!(f, "{}", vn.as_ref().display(info)),
            SimpleValue::Const(v) => write!(f, "{:#x}", v),
            SimpleValue::Mul(MulExpr(a, b)) => {
                write!(
                    f,
                    "({}*{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValue::Add(AddExpr(a, b)) => {
                write!(
                    f,
                    "({}+{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValue::Sub(SubExpr(a, b)) => {
                write!(
                    f,
                    "({}-{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValue::Or(Or(a, b)) => {
                write!(
                    f,
                    "({}||{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValue::Load(Load(a)) => write!(f, "Load({})", a.as_ref().display(info)),
            SimpleValue::Top => write!(f, "⊤"),
        }
    }
}

impl SimpleValue {
    /// Resolve a VarNode to an existing valuation in the state's direct writes,
    /// to a Const if the VarNode is a constant, or to an Entry if unseen.
    pub fn from_varnode_or_entry(state: &SimpleValuationState, vn: &VarNode) -> Self {
        if vn.space_index == VarNode::CONST_SPACE_INDEX {
            SimpleValue::const_(vn.offset as i64)
        } else if let Some(v) = state.valuation.direct_writes.get(vn) {
            v.clone()
        } else {
            SimpleValue::Entry(Entry(Intern::new(vn.clone())))
        }
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
