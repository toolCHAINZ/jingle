// Refactored SimpleValue:
//  - constants are now represented as `VarNode`s in the constant space (keeps offset+size)
//  - every expression node carries a `size: usize` (derived from leaves / consts)
// This file preserves the external API where possible (e.g. `const_(i64)` still exists)
// but internally stores constants as interned `VarNode`s so sizes propagate through expressions.

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

/// Wrap a varnode in an interned container for entry variant.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Entry(pub Intern<VarNode>);

/// Binary expression nodes now carry an explicit `size` field so every node
/// in the expression tree has an associated byte size.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct MulExpr(pub Intern<SimpleValue>, pub Intern<SimpleValue>, pub usize);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AddExpr(pub Intern<SimpleValue>, pub Intern<SimpleValue>, pub usize);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SubExpr(pub Intern<SimpleValue>, pub Intern<SimpleValue>, pub usize);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Or(pub Intern<SimpleValue>, pub Intern<SimpleValue>, pub usize);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Load(pub Intern<SimpleValue>, pub usize);

/// Symbolic valuation built from varnodes and constants (constants are interned VarNodes).
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SimpleValue {
    /// A direct entry referencing an existing non-const varnode
    Entry(Entry),

    /// A constant represented as an interned `VarNode` in the constant space.
    /// This preserves both the offset (value) and the size in bytes.
    Const(Intern<VarNode>),

    /// Binary operators now include an explicit size (in bytes)
    Mul(MulExpr),
    Add(AddExpr),
    Sub(SubExpr),

    Or(Or),
    Load(Load),

    Top,
}

impl SimpleValue {
    /// Return a reference to the `VarNode` if this is a `Const` variant.
    /// This lets callers inspect both offset and size directly.
    pub fn as_const(&self) -> Option<&VarNode> {
        match self {
            SimpleValue::Const(vn_intern) => Some(vn_intern.as_ref()),
            _ => None,
        }
    }

    /// Legacy-style convenience: return the constant value as `i64` (signed).
    /// This preserves the previous numeric-as-`as_const()` behavior for callers
    /// that want the value directly.
    pub fn as_const_value(&self) -> Option<i64> {
        self.as_const().map(|vn| vn.offset as i64)
    }

    /// Accessor for `Entry` variant.
    pub fn as_entry(&self) -> Option<&Entry> {
        match self {
            SimpleValue::Entry(e) => Some(e),
            _ => None,
        }
    }

    /// Accessor for `Mul` variant.
    pub fn as_mul(&self) -> Option<&MulExpr> {
        match self {
            SimpleValue::Mul(m) => Some(m),
            _ => None,
        }
    }

    /// Accessor for `Add` variant.
    pub fn as_add(&self) -> Option<&AddExpr> {
        match self {
            SimpleValue::Add(a) => Some(a),
            _ => None,
        }
    }

    /// Accessor for `Sub` variant.
    pub fn as_sub(&self) -> Option<&SubExpr> {
        match self {
            SimpleValue::Sub(s) => Some(s),
            _ => None,
        }
    }

    /// Accessor for `Or` variant.
    pub fn as_or(&self) -> Option<&Or> {
        match self {
            SimpleValue::Or(o) => Some(o),
            _ => None,
        }
    }

    /// Accessor for `Load` variant.
    pub fn as_load(&self) -> Option<&Load> {
        match self {
            SimpleValue::Load(l) => Some(l),
            _ => None,
        }
    }

    /// Get the size in bytes represented by this SimpleValue.
    /// For `Entry` and `Const`, this returns the underlying VarNode's size.
    /// For composite nodes, the stored size is returned.
    pub fn size(&self) -> usize {
        match self {
            SimpleValue::Entry(Entry(vn)) => vn.as_ref().size,
            SimpleValue::Const(vn) => vn.as_ref().size,
            SimpleValue::Mul(MulExpr(_, _, s))
            | SimpleValue::Add(AddExpr(_, _, s))
            | SimpleValue::Sub(SubExpr(_, _, s))
            | SimpleValue::Or(Or(_, _, s)) => *s,
            SimpleValue::Load(Load(_, s)) => *s,
            SimpleValue::Top => 8, // conservative default
        }
    }

    // --- Convenience constructors -------------------------------------------------

    /// Construct an `Entry(...)` from a `VarNode`.
    pub fn entry(vn: VarNode) -> Self {
        SimpleValue::Entry(Entry(Intern::new(vn)))
    }

    /// Construct a `Const(...)` from a raw i64 value.
    /// We create a `VarNode` in the constant space with a default size of 8 bytes
    /// (64-bit) unless callers use `make_const` to specify a size explicitly.
    pub fn const_(v: i64) -> Self {
        // default to 8-byte sized constant
        let vn = VarNode {
            space_index: VarNode::CONST_SPACE_INDEX,
            offset: v as u64,
            size: 8,
        };
        SimpleValue::Const(Intern::new(vn))
    }

    /// Construct a `Const(...)` directly from a `VarNode` (already contains size).
    pub fn const_from_varnode(vn: VarNode) -> Self {
        SimpleValue::Const(Intern::new(vn))
    }

    /// Construct an `Or(...)` node from two children. Size is derived from children.
    pub fn or(left: SimpleValue, right: SimpleValue) -> Self {
        let s = std::cmp::max(left.size(), right.size());
        SimpleValue::Or(Or(Intern::new(left), Intern::new(right), s))
    }

    /// Construct a `Load(...)` node from a child. Size is taken from the child by default.
    /// (In practice the output varnode size often dictates the load size; callers may
    /// want to construct loads via `make_load_with_size` if available.)
    pub fn load(child: SimpleValue) -> Self {
        let s = child.size();
        SimpleValue::Load(Load(Intern::new(child), s))
    }

    // Keep the older helpers (used by some simplifications) for parity:

    /// Create a constant SimpleValue with the given value and size (in bytes).
    fn make_const(value: i64, size: usize) -> Self {
        let vn = VarNode {
            space_index: VarNode::CONST_SPACE_INDEX,
            offset: value as u64,
            size,
        };
        SimpleValue::Const(Intern::new(vn))
    }

    /// Helper to pick a reasonable size for a new constant when folding results.
    /// Prefer sizes found on any child; fall back to 8 bytes (64-bit).
    fn derive_size_from(val: &SimpleValue) -> usize {
        // If we have an explicit size on this node or on a leaf varnode, return it.
        let s = val.size();
        if s == 0 { 8 } else { s }
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
        let s = std::cmp::max(self.size(), rhs.size());
        SimpleValue::Mul(MulExpr(Intern::new(self), Intern::new(rhs), s))
    }
}

impl Add for SimpleValue {
    type Output = SimpleValue;

    fn add(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        SimpleValue::Add(AddExpr(Intern::new(self), Intern::new(rhs), s))
    }
}

impl Sub for SimpleValue {
    type Output = SimpleValue;

    fn sub(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        SimpleValue::Sub(SubExpr(Intern::new(self), Intern::new(rhs), s))
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

        // both const -> fold using signed wrapping arithmetic consistent with prior behavior
        if let (Some(a_vn), Some(b_vn)) = (a_s.as_const(), b_s.as_const()) {
            let a = a_vn.offset as i64;
            let b = b_vn.offset as i64;
            let res = a.wrapping_add(b);
            let size = SimpleValue::derive_size_from(&a_s).max(SimpleValue::derive_size_from(&b_s));
            return SimpleValue::make_const(res, size);
        }

        // normalization: ensure constants are on the right
        let (left, right) = SimpleValue::normalize_commutative(a_s, b_s);

        // expr + 0 -> expr
        // expr + (- |a|) -> expr - a
        match right.as_const().map(|vn| vn.offset as i64) {
            Some(0) => {
                return left;
            }
            Some(a) => {
                if a < 0 {
                    let new_const =
                        SimpleValue::make_const(-a, SimpleValue::derive_size_from(&left));
                    let sub = SubExpr(
                        Intern::new(left.clone()),
                        Intern::new(new_const),
                        left.size(),
                    )
                    .simplify();
                    return sub;
                }
            }
            _ => {}
        }

        // ((expr + #a) + #b) -> (expr + #(a + b))
        if let SimpleValue::Add(AddExpr(left_inner_left, left_inner_right, _)) = &left {
            if let Some(inner_right_vn) = left_inner_right.as_ref().as_const() {
                if let Some(right_vn) = right.as_const() {
                    let inner_right_const = inner_right_vn.offset as i64;
                    let right_const = right_vn.offset as i64;
                    let res = inner_right_const.wrapping_add(right_const);
                    let size = std::cmp::max(
                        left_inner_left.as_ref().size(),
                        SimpleValue::derive_size_from(&SimpleValue::make_const(res, 8)),
                    );
                    let new_const = SimpleValue::make_const(res, size);
                    return AddExpr(*left_inner_left, Intern::new(new_const), size).simplify();
                }
            }
        }

        // ((expr - #a) + #b) -> (expr - #(a - b))
        if let SimpleValue::Sub(SubExpr(expr, a, _)) = &left {
            if let Some(a_vn) = a.as_ref().as_const() {
                if let Some(b_vn) = right.as_const() {
                    let a_const = a_vn.offset as i64;
                    let b = b_vn.offset as i64;
                    let res = a_const.wrapping_sub(b);
                    let size =
                        std::cmp::max(expr.as_ref().size(), SimpleValue::derive_size_from(&left));
                    let new_const = SimpleValue::make_const(res, size);
                    return SubExpr(*expr, Intern::new(new_const), size).simplify();
                }
            }
        }

        // default: rebuild with simplified children; size is max of children
        let s = std::cmp::max(left.size(), right.size());
        SimpleValue::Add(AddExpr(Intern::new(left), Intern::new(right), s))
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
        if let (Some(left_vn), Some(right_vn)) = (a_s.as_const(), b_s.as_const()) {
            let left = left_vn.offset as i64;
            let right = right_vn.offset as i64;
            let res = left.wrapping_sub(right);
            let size = SimpleValue::derive_size_from(&a_s).max(SimpleValue::derive_size_from(&b_s));
            return SimpleValue::make_const(res, size);
        }

        // normalization: ensure constants are on the right
        let (left, right) = SimpleValue::normalize_commutative(a_s, b_s);

        // expr - 0 -> expr
        // expr - (- |a|) -> expr + a
        match right.as_const().map(|vn| vn.offset as i64) {
            Some(0) => {
                return left;
            }
            Some(a) => {
                if a < 0 {
                    let new_const =
                        SimpleValue::make_const(-a, SimpleValue::derive_size_from(&left));
                    let add = AddExpr(
                        Intern::new(left.clone()),
                        Intern::new(new_const),
                        left.size(),
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
        if let SimpleValue::Add(AddExpr(expr, a, _)) = &left {
            if let Some(a_vn) = a.as_ref().as_const() {
                if let Some(b_vn) = right.as_const() {
                    let a_val = a_vn.offset as i64;
                    let b_val = b_vn.offset as i64;
                    let res = a_val.wrapping_sub(b_val);
                    let size =
                        std::cmp::max(expr.as_ref().size(), SimpleValue::derive_size_from(&left));
                    let new_const = SimpleValue::make_const(res, size);
                    return AddExpr(*expr, Intern::new(new_const), size).simplify();
                }
            }
        }

        // ((expr - #a) - #b) -> (expr - #(a + b))
        if let SimpleValue::Sub(SubExpr(expr, a, _)) = &left {
            if let Some(a_vn) = a.as_ref().as_const() {
                if let Some(b_vn) = right.as_const() {
                    let a_val = a_vn.offset as i64;
                    let b_val = b_vn.offset as i64;
                    let res = a_val.wrapping_add(b_val);
                    let size =
                        std::cmp::max(expr.as_ref().size(), SimpleValue::derive_size_from(&left));
                    let new_const = SimpleValue::make_const(res, size);
                    return SubExpr(*expr, Intern::new(new_const), size).simplify();
                }
            }
        }

        let s = std::cmp::max(left.size(), right.size());
        SimpleValue::Sub(SubExpr(Intern::new(left), Intern::new(right), s))
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
        if let (Some(a_vn), Some(b_vn)) = (left.as_const(), right.as_const()) {
            let a_v = a_vn.offset as i64;
            let b_v = b_vn.offset as i64;
            let res = a_v.wrapping_mul(b_v);
            let size =
                SimpleValue::derive_size_from(&left).max(SimpleValue::derive_size_from(&right));
            return SimpleValue::make_const(res, size);
        }

        // expr * 1 -> expr
        if right.as_const().map(|vn| vn.offset as i64) == Some(1) {
            return left;
        }

        // expr * 0 -> 0
        if right.as_const().map(|vn| vn.offset as i64) == Some(0) {
            let size = SimpleValue::derive_size_from(&left);
            return SimpleValue::make_const(0, size);
        }

        let s = std::cmp::max(left.size(), right.size());
        SimpleValue::Mul(MulExpr(Intern::new(left), Intern::new(right), s))
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
        if let SimpleValue::Or(Or(inner_a, inner_b, _)) = &right {
            if inner_a.as_ref() == &left {
                let inner = SimpleValue::Or(Or(Intern::new(left.clone()), *inner_b, right.size()))
                    .simplify();
                return inner;
            }
            if inner_b.as_ref() == &left {
                let inner = SimpleValue::Or(Or(Intern::new(left.clone()), *inner_a, right.size()))
                    .simplify();
                return inner;
            }
        }

        // Factor common child between two Ors:
        // Or(Or(a,b), Or(a,c)) -> Or(a, Or(b,c)) and symmetric variants.
        if let (SimpleValue::Or(Or(l1, l2, _)), SimpleValue::Or(Or(r1, r2, _))) = (&left, &right) {
            // check all combinations for equal common child
            if l1.as_ref() == r1.as_ref() {
                let inner = SimpleValue::Or(Or(
                    *l2,
                    *r2,
                    std::cmp::max(l2.as_ref().size(), r2.as_ref().size()),
                ))
                .simplify();
                let s = std::cmp::max(l1.as_ref().size(), inner.size());
                return SimpleValue::Or(Or(
                    Intern::new(l1.as_ref().clone()),
                    Intern::new(inner),
                    s,
                ))
                .simplify();
            }
            if l1.as_ref() == r2.as_ref() {
                let inner = SimpleValue::Or(Or(
                    *l2,
                    *r1,
                    std::cmp::max(l2.as_ref().size(), r1.as_ref().size()),
                ))
                .simplify();
                let s = std::cmp::max(l1.as_ref().size(), inner.size());
                return SimpleValue::Or(Or(
                    Intern::new(l1.as_ref().clone()),
                    Intern::new(inner),
                    s,
                ))
                .simplify();
            }
            if l2.as_ref() == r1.as_ref() {
                let inner = SimpleValue::Or(Or(
                    *l1,
                    *r2,
                    std::cmp::max(l1.as_ref().size(), r2.as_ref().size()),
                ))
                .simplify();
                let s = std::cmp::max(l2.as_ref().size(), inner.size());
                return SimpleValue::Or(Or(
                    Intern::new(l2.as_ref().clone()),
                    Intern::new(inner),
                    s,
                ))
                .simplify();
            }
            if l2.as_ref() == r2.as_ref() {
                let inner = SimpleValue::Or(Or(
                    *l1,
                    *r1,
                    std::cmp::max(l1.as_ref().size(), r1.as_ref().size()),
                ))
                .simplify();
                let s = std::cmp::max(l2.as_ref().size(), inner.size());
                return SimpleValue::Or(Or(
                    Intern::new(l2.as_ref().clone()),
                    Intern::new(inner),
                    s,
                ))
                .simplify();
            }
        }

        // default: rebuild with simplified children
        let s = std::cmp::max(left.size(), right.size());
        SimpleValue::Or(Or(Intern::new(left), Intern::new(right), s))
    }
}

impl Simplify for Load {
    fn simplify(&self) -> SimpleValue {
        let a_intern = self.0;
        let a_s = a_intern.as_ref().simplify();

        if matches!(a_s, SimpleValue::Top) {
            return SimpleValue::Top;
        }

        // keep the same size as recorded on this Load node
        SimpleValue::Load(Load(Intern::new(a_s), self.1))
    }
}

impl JingleDisplay for SimpleValue {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        match self {
            SimpleValue::Entry(Entry(vn)) => write!(f, "{}", vn.as_ref().display(info)),
            SimpleValue::Const(vn) => {
                // print constant offset in hex (retain prior appearance)
                write!(f, "{:#x}", vn.as_ref().offset)
            }
            SimpleValue::Mul(MulExpr(a, b, _)) => {
                write!(
                    f,
                    "({}*{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValue::Add(AddExpr(a, b, _)) => {
                write!(
                    f,
                    "({}+{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValue::Sub(SubExpr(a, b, _)) => {
                write!(
                    f,
                    "({}-{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValue::Or(Or(a, b, _)) => {
                write!(
                    f,
                    "({}||{})",
                    a.as_ref().display(info),
                    b.as_ref().display(info)
                )
            }
            SimpleValue::Load(Load(a, _)) => write!(f, "Load({})", a.as_ref().display(info)),
            SimpleValue::Top => write!(f, "⊤"),
        }
    }
}

impl std::fmt::Display for SimpleValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimpleValue::Entry(Entry(vn)) => {
                // Delegate to VarNode's Display implementation
                write!(f, "{}", vn.as_ref())
            }
            SimpleValue::Const(vn) => {
                // Print constant offset in hex (consistent with jingle display)
                write!(f, "{:#x}", vn.as_ref().offset)
            }
            SimpleValue::Mul(MulExpr(a, b, _)) => {
                // Infix multiplication with parens
                write!(f, "({}*{})", a.as_ref(), b.as_ref())
            }
            SimpleValue::Add(AddExpr(a, b, _)) => {
                // Infix addition with parens
                write!(f, "({}+{})", a.as_ref(), b.as_ref())
            }
            SimpleValue::Sub(SubExpr(a, b, _)) => {
                // Infix subtraction with parens
                write!(f, "({}-{})", a.as_ref(), b.as_ref())
            }
            SimpleValue::Or(Or(a, b, _)) => {
                // Logical-or style with double pipes
                write!(f, "({}||{})", a.as_ref(), b.as_ref())
            }
            SimpleValue::Load(Load(a, _)) => {
                // Load(child)
                write!(f, "Load({})", a.as_ref())
            }
            SimpleValue::Top => {
                // Special top symbol
                write!(f, "⊤")
            }
        }
    }
}

impl std::fmt::LowerHex for SimpleValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimpleValue::Entry(Entry(vn)) => {
                // VarNode doesn't implement LowerHex; fall back to Display
                write!(f, "{}", vn.as_ref())
            }
            SimpleValue::Const(vn) => {
                // Lower-hex for constants: no 0x prefix, lowercase hex digits
                write!(f, "{:x}", vn.as_ref().offset)
            }
            SimpleValue::Mul(MulExpr(a, b, _)) => {
                write!(f, "({:x}*{:x})", a.as_ref(), b.as_ref())
            }
            SimpleValue::Add(AddExpr(a, b, _)) => {
                write!(f, "({:x}+{:x})", a.as_ref(), b.as_ref())
            }
            SimpleValue::Sub(SubExpr(a, b, _)) => {
                write!(f, "({:x}-{:x})", a.as_ref(), b.as_ref())
            }
            SimpleValue::Or(Or(a, b, _)) => {
                write!(f, "({:x}||{:x})", a.as_ref(), b.as_ref())
            }
            SimpleValue::Load(Load(a, _)) => {
                write!(f, "Load({:x})", a.as_ref())
            }
            SimpleValue::Top => write!(f, "⊤"),
        }
    }
}

impl SimpleValue {
    /// Resolve a VarNode to an existing valuation in the state's direct writes,
    /// to a Const if the VarNode is a constant, or to an Entry if unseen.
    pub fn from_varnode_or_entry(state: &SimpleValuationState, vn: &VarNode) -> Self {
        if vn.space_index == VarNode::CONST_SPACE_INDEX {
            // preserve the size of the incoming varnode
            SimpleValue::Const(Intern::new(vn.clone()))
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
