use crate::{
    analysis::{cpa::lattice::JoinSemiLattice, valuation::ValuationState},
    display::JingleDisplay,
};
use internment::Intern;
use jingle_sleigh::{SleighArchInfo, VarNode};
use std::{
    borrow::Borrow,
    ops::{BitAnd, BitXor, Deref},
};
use std::{
    fmt::Formatter,
    ops::{Add, Mul, Sub},
};

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Value {}
    impl Sealed for internment::Intern<super::Value> {}
    impl Sealed for &super::Value {}
    impl Sealed for &internment::Intern<super::Value> {}
}

/// Anything that can be used as an operand to a `Value` constructor.
///
/// Implemented for:
/// - `Value` — takes ownership and interns
/// - `Intern<Value>` — already interned; identity conversion (free copy)
/// - `&Value` — clones and interns
///
/// This trait is sealed; external implementations are not supported.
pub trait IntoInternedValue: sealed::Sealed {
    fn into_interned(self) -> Intern<Value>;
}

impl IntoInternedValue for Value {
    fn into_interned(self) -> Intern<Value> {
        Intern::new(self)
    }
}

impl IntoInternedValue for Intern<Value> {
    fn into_interned(self) -> Intern<Value> {
        self
    }
}

impl IntoInternedValue for &Value {
    fn into_interned(self) -> Intern<Value> {
        Intern::new(self.clone())
    }
}

impl IntoInternedValue for &Intern<Value> {
    fn into_interned(self) -> Intern<Value> {
        *self
    }
}

trait Simplify {
    fn simplify(&self) -> Value;
}

/// An entry value of a direct location
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Entry(VarNode);

impl Deref for Entry {
    type Target = VarNode;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A constant value
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Const(VarNode);

impl From<VarNode> for Const {
    fn from(value: VarNode) -> Self {
        Self(value)
    }
}

impl Deref for Const {
    type Target = VarNode;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A value representing a positive offset from a location pointed to by another value.
/// This is similar to sleigh/ghidra's post-analysis stack offset space.
///
/// The const in here has some special semantics associated with it:
/// Though a member of the CONST space, its size represents the number of bytes
/// covered, not the size of the representation of constant itself.
///
/// For example, `Offset(r1, 4:8)` refers to the range of 8 bytes that begins
/// 4 bytes after the address pointed to by r1.
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Offset(Intern<Entry>, Intern<Const>);

impl Offset {
    pub fn new(base: impl Borrow<Entry>, offset: impl Borrow<Const>) -> Self {
        Self(
            Intern::new(base.borrow().clone()),
            Intern::new(offset.borrow().clone()),
        )
    }

    pub fn base_vn(&self) -> &Entry {
        self.0.as_ref()
    }

    pub fn offset(&self) -> &Const {
        self.1.as_ref()
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        // Two offsets overlap if they refer to the same base and their offset ranges intersect.
        if self.base_vn() != other.base_vn() {
            return false;
        }
        let self_start = self.offset().as_ref().offset();
        let self_end = self_start + self.offset().as_ref().size() as u64;
        let other_start = other.offset().as_ref().offset();
        let other_end = other_start + other.offset().as_ref().size() as u64;

        // Check if the ranges [self_start, self_end) and [other_start, other_end) overlap
        !(self_end <= other_start || other_end <= self_start)
    }
}

/// A multiplication expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct MulExpr(pub Intern<Value>, pub Intern<Value>, pub usize);

/// An addition expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct AddExpr(pub Intern<Value>, pub Intern<Value>, pub usize);

/// A subtraction expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct SubExpr(pub Intern<Value>, pub Intern<Value>, pub usize);

/// An expression representing two possible values (abstract interpretation choice)
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Choice(pub Intern<Value>, pub Intern<Value>, pub usize);

/// A bitwise XOR expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct XorExpr(pub Intern<Value>, pub Intern<Value>, pub usize);

/// A bitwise OR expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct OrExpr(pub Intern<Value>, pub Intern<Value>, pub usize);

/// A bitwise AND expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct AndExpr(pub Intern<Value>, pub Intern<Value>, pub usize);

/// A left shift expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IntLeftShiftExpr(pub Intern<Value>, pub Intern<Value>, pub usize);

/// An unsigned right shift expression (logical shift)
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IntRightShiftExpr(pub Intern<Value>, pub Intern<Value>, pub usize);

/// A signed right shift expression (arithmetic shift)
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IntSignedRightShiftExpr(pub Intern<Value>, pub Intern<Value>, pub usize);

/// A signed comparison operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IntSLess(pub Intern<Value>, pub Intern<Value>);

/// An equality comparison operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IntEqual(pub Intern<Value>, pub Intern<Value>);

/// An unsigned comparison operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IntLess(pub Intern<Value>, pub Intern<Value>);

/// A PopCount operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct PopCount(pub Intern<Value>);

/// A two's complement operator (INT_2COMP): computes -x = ~x + 1
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Int2CompExpr(pub Intern<Value>, pub usize);

/// An inequality comparison operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IntNotEqual(pub Intern<Value>, pub Intern<Value>);

/// An unsigned less-than-or-equal comparison operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IntLessEqual(pub Intern<Value>, pub Intern<Value>);

/// A signed less-than-or-equal comparison operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IntSLessEqual(pub Intern<Value>, pub Intern<Value>);

/// Unsigned addition carry-out
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IntCarry(pub Intern<Value>, pub Intern<Value>);

/// Signed addition overflow (SCARRY)
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IntSCarry(pub Intern<Value>, pub Intern<Value>);

/// Signed subtraction overflow (SBORROW)
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IntSBorrow(pub Intern<Value>, pub Intern<Value>);

/// A load of a certain size from a pointer with a certain value
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Load(pub Intern<Value>, pub usize);

/// A zero-extension of the inner value to `output_size` bytes
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct ZeroExtend(pub Intern<Value>, pub usize);

/// A sign-extension of the inner value to `output_size` bytes
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct SignExtend(pub Intern<Value>, pub usize);

/// Extraction of `output_size` bytes from the inner value starting at `byte_offset`
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Extract(pub Intern<Value>, pub usize, pub usize);

impl AsRef<VarNode> for Const {
    fn as_ref(&self) -> &VarNode {
        &self.0
    }
}

impl AsRef<VarNode> for Entry {
    fn as_ref(&self) -> &VarNode {
        &self.0
    }
}

/// Symbolic valuation built from varnodes and constants (constants are interned VarNodes).
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum Value {
    /// A direct entry referencing an existing non-const varnode
    Entry(Entry),

    /// A constant represented as an interned `VarNode` in the constant space.
    /// This preserves both the offset (value) and the size in bytes.
    Const(Const),

    Offset(Offset),
    /// Binary operators now include an explicit size (in bytes)
    Mul(MulExpr),
    Add(AddExpr),
    Sub(SubExpr),

    Choice(Choice),
    Xor(XorExpr),
    Or(OrExpr),
    And(AndExpr),
    IntLeftShift(IntLeftShiftExpr),
    IntRightShift(IntRightShiftExpr),
    IntSignedRightShift(IntSignedRightShiftExpr),
    Load(Load),

    ZeroExtend(ZeroExtend),
    SignExtend(SignExtend),
    Extract(Extract),

    IntSLess(IntSLess),
    IntEqual(IntEqual),
    IntLess(IntLess),
    PopCount(PopCount),
    Int2Comp(Int2CompExpr),

    IntNotEqual(IntNotEqual),
    IntLessEqual(IntLessEqual),
    IntSLessEqual(IntSLessEqual),
    IntCarry(IntCarry),
    IntSCarry(IntSCarry),
    IntSBorrow(IntSBorrow),

    Top,
}

impl Value {
    /// Return a reference to the `VarNode` if this is a `Const` variant.
    /// This lets callers inspect both offset and size directly.
    pub fn as_const(&self) -> Option<&VarNode> {
        match self {
            Value::Const(vn_intern) => Some(vn_intern.as_ref()),
            _ => None,
        }
    }

    /// Legacy-style convenience: return the constant value as `i64` (signed).
    /// This preserves the previous numeric-as-`as_const()` behavior for callers
    /// that want the value directly.
    pub fn as_const_value(&self) -> Option<i64> {
        self.as_const().map(|vn| vn.offset() as i64)
    }

    /// Accessor for `Entry` variant.
    pub fn as_entry(&self) -> Option<&Entry> {
        match self {
            Value::Entry(e) => Some(e),
            _ => None,
        }
    }

    /// Accessor for `Entry` variant.
    pub fn as_offset(&self) -> Option<&Offset> {
        match self {
            Value::Offset(e) => Some(e),
            _ => None,
        }
    }

    fn is_compound(&self) -> bool {
        matches!(
            self,
            Value::Mul(_)
                | Value::Add(_)
                | Value::Sub(_)
                | Value::Choice(_)
                | Value::Xor(_)
                | Value::Or(_)
                | Value::And(_)
                | Value::IntLeftShift(_)
                | Value::IntRightShift(_)
                | Value::IntSignedRightShift(_)
                | Value::ZeroExtend(_)
                | Value::SignExtend(_)
                | Value::Extract(_)
                | Value::IntSLess(_)
                | Value::IntEqual(_)
                | Value::IntLess(_)
                | Value::PopCount(_)
                | Value::Int2Comp(_)
                | Value::IntNotEqual(_)
                | Value::IntLessEqual(_)
                | Value::IntSLessEqual(_)
                | Value::IntCarry(_)
                | Value::IntSCarry(_)
                | Value::IntSBorrow(_)
        )
    }

    /// Accessor for `Mul` variant.
    pub fn as_mul(&self) -> Option<&MulExpr> {
        match self {
            Value::Mul(m) => Some(m),
            _ => None,
        }
    }

    /// Accessor for `Add` variant.
    pub fn as_add(&self) -> Option<&AddExpr> {
        match self {
            Value::Add(a) => Some(a),
            _ => None,
        }
    }

    /// Accessor for `Sub` variant.
    pub fn as_sub(&self) -> Option<&SubExpr> {
        match self {
            Value::Sub(s) => Some(s),
            _ => None,
        }
    }

    /// Accessor for `Choice` variant.
    pub fn as_choice(&self) -> Option<&Choice> {
        match self {
            Value::Choice(o) => Some(o),
            _ => None,
        }
    }

    /// Accessor for `Xor` variant.
    pub fn as_xor(&self) -> Option<&XorExpr> {
        match self {
            Value::Xor(x) => Some(x),
            _ => None,
        }
    }

    /// Accessor for `Or` variant (bitwise OR).
    pub fn as_or(&self) -> Option<&OrExpr> {
        match self {
            Value::Or(o) => Some(o),
            _ => None,
        }
    }

    /// Accessor for `And` variant.
    pub fn as_and(&self) -> Option<&AndExpr> {
        match self {
            Value::And(a) => Some(a),
            _ => None,
        }
    }

    /// Accessor for `Load` variant.
    pub fn as_load(&self) -> Option<&Load> {
        match self {
            Value::Load(l) => Some(l),
            _ => None,
        }
    }

    /// Accessor for `IntSLess` variant.
    pub fn as_int_sless(&self) -> Option<&IntSLess> {
        match self {
            Value::IntSLess(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `IntEqual` variant.
    pub fn as_int_equal(&self) -> Option<&IntEqual> {
        match self {
            Value::IntEqual(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `IntLess` variant.
    pub fn as_int_less(&self) -> Option<&IntLess> {
        match self {
            Value::IntLess(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `PopCount` variant.
    pub fn as_popcount(&self) -> Option<&PopCount> {
        match self {
            Value::PopCount(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `Int2Comp` variant.
    pub fn as_int_2comp(&self) -> Option<&Int2CompExpr> {
        match self {
            Value::Int2Comp(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `IntNotEqual` variant.
    pub fn as_int_not_equal(&self) -> Option<&IntNotEqual> {
        match self {
            Value::IntNotEqual(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `IntLessEqual` variant.
    pub fn as_int_less_equal(&self) -> Option<&IntLessEqual> {
        match self {
            Value::IntLessEqual(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `IntSLessEqual` variant.
    pub fn as_int_sless_equal(&self) -> Option<&IntSLessEqual> {
        match self {
            Value::IntSLessEqual(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `IntCarry` variant.
    pub fn as_int_carry(&self) -> Option<&IntCarry> {
        match self {
            Value::IntCarry(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `IntSCarry` variant.
    pub fn as_int_scarry(&self) -> Option<&IntSCarry> {
        match self {
            Value::IntSCarry(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `IntSBorrow` variant.
    pub fn as_int_sborrow(&self) -> Option<&IntSBorrow> {
        match self {
            Value::IntSBorrow(v) => Some(v),
            _ => None,
        }
    }

    /// Get the size in bytes represented by this Value.
    /// For `Entry` and `Const`, this returns the underlying VarNode's size.
    /// For composite nodes, the stored size is returned.
    pub fn size(&self) -> usize {
        match self {
            Value::Entry(Entry(vn)) => vn.size(),
            Value::Const(vn) => vn.as_ref().size(),
            Value::Offset(Offset(_, vn)) => vn.as_ref().0.size(),
            Value::Mul(MulExpr(_, _, s))
            | Value::Add(AddExpr(_, _, s))
            | Value::Sub(SubExpr(_, _, s))
            | Value::Choice(Choice(_, _, s))
            | Value::Xor(XorExpr(_, _, s))
            | Value::Or(OrExpr(_, _, s))
            | Value::And(AndExpr(_, _, s))
            | Value::IntLeftShift(IntLeftShiftExpr(_, _, s))
            | Value::IntRightShift(IntRightShiftExpr(_, _, s))
            | Value::IntSignedRightShift(IntSignedRightShiftExpr(_, _, s)) => *s,
            Value::Load(Load(_, s)) => *s,
            Value::ZeroExtend(ZeroExtend(_, s)) | Value::SignExtend(SignExtend(_, s)) => *s,
            Value::Extract(Extract(_, _, s)) => *s,
            Value::IntSLess(_)
            | Value::IntEqual(_)
            | Value::IntLess(_)
            | Value::PopCount(_)
            | Value::IntNotEqual(_)
            | Value::IntLessEqual(_)
            | Value::IntSLessEqual(_)
            | Value::IntCarry(_)
            | Value::IntSCarry(_)
            | Value::IntSBorrow(_) => 1,
            Value::Int2Comp(Int2CompExpr(_, s)) => *s,
            Value::Top => 8, // conservative default
        }
    }

    // --- Convenience constructors -------------------------------------------------

    /// Construct an `Entry(...)` from a `VarNode`.
    pub fn entry(vn: VarNode) -> Self {
        Value::Entry(Entry(vn))
    }

    /// Construct an `Entry(...)` from a `VarNode`.
    pub fn offset(vn: VarNode, offset: VarNode) -> Self {
        Value::Offset(Offset(Intern::new(Entry(vn)), Intern::new(Const(offset))))
    }

    /// Construct a `Const(...)` from a raw i64 value.
    /// We create a `VarNode` in the constant space with a default size of 8 bytes
    /// (64-bit) unless callers use `make_const` to specify a size explicitly.
    pub fn const_(v: i64) -> Self {
        // default to 8-byte sized constant
        let vn = VarNode::new_const(v as u64, 8u32);
        Value::Const(Const(vn))
    }

    /// Construct a `Const(...)` directly from a `VarNode` (already contains size).
    pub fn const_from_varnode(vn: VarNode) -> Self {
        Value::Const(Const(vn))
    }

    /// Construct a `Choice(...)` node from two children. Size is derived from children.
    /// This represents an abstract interpretation choice between two possible values.
    pub fn choice(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        let left = left.into_interned();
        let right = right.into_interned();
        let s = std::cmp::max(left.size(), right.size());
        Value::Choice(Choice(left, right, s))
    }

    /// Construct a `Xor(...)` node from two children. Size is derived from children.
    pub fn xor(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        let left = left.into_interned();
        let right = right.into_interned();
        let s = std::cmp::max(left.size(), right.size());
        Value::Xor(XorExpr(left, right, s))
    }

    /// Construct an `Or(...)` node from two children (bitwise OR). Size is derived from children.
    pub fn or(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        let left = left.into_interned();
        let right = right.into_interned();
        let s = std::cmp::max(left.size(), right.size());
        Value::Or(OrExpr(left, right, s))
    }

    /// Construct an `And(...)` node from two children. Size is derived from children.
    pub fn and(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        let left = left.into_interned();
        let right = right.into_interned();
        let s = std::cmp::max(left.size(), right.size());
        Value::And(AndExpr(left, right, s))
    }

    /// Construct a `Load(...)` node from a child. Size is taken from the child by default.
    /// (In practice the output varnode size often dictates the load size; callers may
    /// want to construct loads via `make_load_with_size` if available.)
    /// todo: we should _not_ be pulling the size from the child value; it is independent of
    /// pointer size
    pub fn load(child: impl IntoInternedValue) -> Self {
        let child = child.into_interned();
        let s = child.size();
        Value::Load(Load(child, s))
    }

    /// Construct an `IntEqual(...)` node from two children.
    pub fn int_equal(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        Value::IntEqual(IntEqual(left.into_interned(), right.into_interned()))
    }

    /// Construct an `IntLess(...)` node from two children.
    pub fn int_less(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        Value::IntLess(IntLess(left.into_interned(), right.into_interned()))
    }

    /// Construct an `IntSLess(...)` node from two children.
    pub fn int_sless(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        Value::IntSLess(IntSLess(left.into_interned(), right.into_interned()))
    }

    /// Construct a `PopCount(...)` node from a child.
    pub fn popcount(child: impl IntoInternedValue) -> Self {
        Value::PopCount(PopCount(child.into_interned()))
    }

    /// Construct an `Int2Comp(...)` node from a child.
    pub fn int_2comp(child: impl IntoInternedValue) -> Self {
        let child = child.into_interned();
        let s = child.size();
        Value::Int2Comp(Int2CompExpr(child, s))
    }

    /// Construct an `IntNotEqual(...)` node from two children.
    pub fn int_not_equal(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        Value::IntNotEqual(IntNotEqual(left.into_interned(), right.into_interned()))
    }

    /// Construct an `IntLessEqual(...)` node from two children.
    pub fn int_less_equal(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        Value::IntLessEqual(IntLessEqual(left.into_interned(), right.into_interned()))
    }

    /// Construct an `IntSLessEqual(...)` node from two children.
    pub fn int_sless_equal(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        Value::IntSLessEqual(IntSLessEqual(left.into_interned(), right.into_interned()))
    }

    /// Construct an `IntCarry(...)` node from two children.
    pub fn int_carry(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        Value::IntCarry(IntCarry(left.into_interned(), right.into_interned()))
    }

    /// Construct an `IntSCarry(...)` node from two children.
    pub fn int_scarry(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        Value::IntSCarry(IntSCarry(left.into_interned(), right.into_interned()))
    }

    /// Construct an `IntSBorrow(...)` node from two children.
    pub fn int_sborrow(left: impl IntoInternedValue, right: impl IntoInternedValue) -> Self {
        Value::IntSBorrow(IntSBorrow(left.into_interned(), right.into_interned()))
    }

    /// Construct a `ZeroExtend(...)` node that zero-extends `inner` to `output_size` bytes.
    pub fn zero_extend(inner: impl IntoInternedValue, output_size: usize) -> Self {
        Value::ZeroExtend(ZeroExtend(inner.into_interned(), output_size))
    }

    /// Construct a `SignExtend(...)` node that sign-extends `inner` to `output_size` bytes.
    pub fn sign_extend(inner: impl IntoInternedValue, output_size: usize) -> Self {
        Value::SignExtend(SignExtend(inner.into_interned(), output_size))
    }

    /// Construct an `Extract(...)` node that extracts `output_size` bytes from `inner`
    /// starting at `byte_offset`.
    pub fn extract(inner: impl IntoInternedValue, byte_offset: usize, output_size: usize) -> Self {
        Value::Extract(Extract(inner.into_interned(), byte_offset, output_size))
    }

    // Keep the older helpers (used by some simplifications) for parity:

    /// Create a constant Value with the given value and size (in bytes).
    fn make_const(value: i64, size: u32) -> Self {
        let vn = VarNode::new_const(value as u64, size);
        Value::Const(Const(vn))
    }

    /// Helper to pick a reasonable size for a new constant when folding results.
    /// Prefer sizes found on any child; fall back to 8 bytes (64-bit).
    fn derive_size_from(val: &Value) -> usize {
        // If we have an explicit size on this node or on a leaf varnode, return it.
        let s = val.size();
        if s == 0 { 8 } else { s }
    }

    /// Normalize commutative operands so that constants (if present) are on the right.
    /// Returns (left, right) possibly swapped.
    fn normalize_commutative(left: Value, right: Value) -> (Value, Value) {
        let left_is_const = left.as_const().is_some();
        let right_is_const = right.as_const().is_some();

        // If left is const and right is not, swap them so constant is on right.
        if left_is_const && !right_is_const {
            (right, left)
        } else {
            (left, right)
        }
    }

    /// Normalize Choice operands so that the canonical form has a non-Choice on the left
    /// and a Choice on the right when one operand is a Choice. This makes simplifications
    /// like `Choice(Choice(a,b), c)` and `Choice(c, Choice(a,b))` handled uniformly.
    fn normalize_choice(left: Value, right: Value) -> (Value, Value) {
        let left_is_choice = matches!(left, Value::Choice(_));
        let right_is_choice = matches!(right, Value::Choice(_));

        // If left is a Choice and right is not, swap so the Choice is on the right.
        if left_is_choice && !right_is_choice {
            (right, left)
        } else {
            (left, right)
        }
    }

    /// Provide a coarse rank for variants so we can produce deterministic ordering
    /// among different kinds of children when canonicalizing binary commutative nodes.
    fn variant_rank(v: &Value) -> u8 {
        match v {
            Value::Const(_) => 0,
            Value::Entry(_) => 1,
            Value::Offset(_) => 2,
            Value::Mul(_) => 3,
            Value::Add(_) => 4,
            Value::Sub(_) => 5,
            Value::Choice(_) => 6,
            Value::Xor(_) => 7,
            Value::Or(_) => 8,
            Value::And(_) => 9,
            Value::IntLeftShift(_) => 10,
            Value::IntRightShift(_) => 11,
            Value::IntSignedRightShift(_) => 12,
            Value::Load(_) => 13,
            Value::ZeroExtend(_) => 14,
            Value::SignExtend(_) => 15,
            Value::Extract(_) => 16,
            Value::Top => 17,
            Value::IntSLess(_) => 18,
            Value::IntEqual(_) => 19,
            Value::IntLess(_) => 20,
            Value::PopCount(_) => 21,
            Value::Int2Comp(_) => 22,
            Value::IntNotEqual(_) => 23,
            Value::IntLessEqual(_) => 24,
            Value::IntSLessEqual(_) => 25,
            Value::IntCarry(_) => 26,
            Value::IntSCarry(_) => 27,
            Value::IntSBorrow(_) => 28,
        }
    }
}

impl Simplify for Value {
    fn simplify(&self) -> Value {
        match self {
            Value::Mul(expr) => expr.simplify(),
            Value::Add(expr) => expr.simplify(),
            Value::Sub(expr) => expr.simplify(),
            Value::Choice(expr) => expr.simplify(),
            Value::Xor(expr) => expr.simplify(),
            Value::Or(expr) => expr.simplify(),
            Value::And(expr) => expr.simplify(),
            Value::IntLeftShift(expr) => expr.simplify(),
            Value::IntRightShift(expr) => expr.simplify(),
            Value::IntSignedRightShift(expr) => expr.simplify(),
            Value::Load(expr) => expr.simplify(),
            Value::ZeroExtend(expr) => expr.simplify(),
            Value::SignExtend(expr) => expr.simplify(),
            Value::Extract(expr) => expr.simplify(),
            Value::IntSLess(expr) => expr.simplify(),
            Value::IntEqual(expr) => expr.simplify(),
            Value::IntLess(expr) => expr.simplify(),
            Value::PopCount(expr) => expr.simplify(),
            Value::Int2Comp(expr) => expr.simplify(),
            Value::IntNotEqual(expr) => expr.simplify(),
            Value::IntLessEqual(expr) => expr.simplify(),
            Value::IntSLessEqual(expr) => expr.simplify(),
            Value::IntCarry(expr) => expr.simplify(),
            Value::IntSCarry(expr) => expr.simplify(),
            Value::IntSBorrow(expr) => expr.simplify(),
            Value::Entry(_) | Value::Offset(_) | Value::Const(_) | Value::Top => self.clone(),
        }
    }
}

impl Mul for Value {
    type Output = Value;

    fn mul(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        Value::Mul(MulExpr(Intern::new(self), Intern::new(rhs), s))
    }
}

impl Add for Value {
    type Output = Value;

    fn add(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        Value::Add(AddExpr(Intern::new(self), Intern::new(rhs), s))
    }
}

impl BitXor for Value {
    type Output = Value;

    fn bitxor(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        Value::Xor(XorExpr(Intern::new(self), Intern::new(rhs), s))
    }
}

impl BitAnd for Value {
    type Output = Value;

    fn bitand(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        Value::And(AndExpr(Intern::new(self), Intern::new(rhs), s))
    }
}

impl std::ops::BitOr for Value {
    type Output = Value;

    fn bitor(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        Value::Or(OrExpr(Intern::new(self), Intern::new(rhs), s))
    }
}

impl Sub for Value {
    type Output = Value;

    fn sub(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        Value::Sub(SubExpr(Intern::new(self), Intern::new(rhs), s))
    }
}

impl Value {
    /// Inherent simplify method so callers don't need the `Simplify` trait in scope.
    /// This delegates to the same per-variant simplifiers that the `Simplify`
    /// implementations provide for the individual AST node structs.
    pub fn simplify(&self) -> Value {
        Simplify::simplify(self)
    }

    /// Recursively substitute `Entry` and `Load` values using a context valuation.
    ///
    /// For `Entry(vn)`: look up `vn` in `context.direct_writes` and substitute if found.
    /// For `Load(ptr, size)`: first substitute `ptr`, then look up the result in
    /// `context.indirect_writes` and substitute if found.
    /// All other variants recursively substitute their children.
    ///
    /// The result is simplified to handle cycles (e.g., A = B and B = A becomes A = A)
    /// and normalize expressions.
    pub fn substitute(&self, context: &crate::analysis::valuation::ValuationSet) -> Value {
        let result = match self {
            // Base cases: Entry and Const
            Value::Entry(Entry(vn)) => {
                // Look up this varnode in context's direct writes
                context
                    .direct_writes
                    .get(vn)
                    .map(|v| {
                        // If substitution results in the same varnode, return original Entry to avoid cycles
                        if v.as_entry().map(|e| e.0) == Some(*vn) {
                            self.clone()
                        } else {
                            // Substitute the found value recursively
                            v.substitute(context)
                        }
                    })
                    .unwrap_or_else(|| self.clone())
            }
            Value::Const(_) => self.clone(),
            Value::Top => Value::Top,

            // Offset: substitute the base entry
            Value::Offset(_) => self.clone(),

            // Load: substitute the pointer, then check if the result is in indirect_writes
            Value::Load(Load(ptr, size)) => {
                let subst_ptr = ptr.as_ref().substitute(context);
                // Look up the substituted pointer in indirect writes
                context
                    .indirect_writes
                    .get(&subst_ptr)
                    .map(|v| v.substitute(context))
                    .unwrap_or_else(|| Value::Load(Load(Intern::new(subst_ptr), *size)))
            }

            // Binary operators: substitute both operands
            Value::Mul(MulExpr(a, b, s)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::Mul(MulExpr(Intern::new(a_subst), Intern::new(b_subst), *s))
            }
            Value::Add(AddExpr(a, b, s)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::Add(AddExpr(Intern::new(a_subst), Intern::new(b_subst), *s))
            }
            Value::Sub(SubExpr(a, b, s)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::Sub(SubExpr(Intern::new(a_subst), Intern::new(b_subst), *s))
            }
            Value::Choice(Choice(a, b, s)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::Choice(Choice(Intern::new(a_subst), Intern::new(b_subst), *s))
            }
            Value::Xor(XorExpr(a, b, s)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::Xor(XorExpr(Intern::new(a_subst), Intern::new(b_subst), *s))
            }
            Value::Or(OrExpr(a, b, s)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::Or(OrExpr(Intern::new(a_subst), Intern::new(b_subst), *s))
            }
            Value::And(AndExpr(a, b, s)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::And(AndExpr(Intern::new(a_subst), Intern::new(b_subst), *s))
            }
            Value::IntLeftShift(IntLeftShiftExpr(a, b, s)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::IntLeftShift(IntLeftShiftExpr(
                    Intern::new(a_subst),
                    Intern::new(b_subst),
                    *s,
                ))
            }
            Value::IntRightShift(IntRightShiftExpr(a, b, s)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::IntRightShift(IntRightShiftExpr(
                    Intern::new(a_subst),
                    Intern::new(b_subst),
                    *s,
                ))
            }
            Value::IntSignedRightShift(IntSignedRightShiftExpr(a, b, s)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::IntSignedRightShift(IntSignedRightShiftExpr(
                    Intern::new(a_subst),
                    Intern::new(b_subst),
                    *s,
                ))
            }

            // Unary operators: substitute the operand
            Value::ZeroExtend(ZeroExtend(inner, size)) => {
                let inner_subst = inner.as_ref().substitute(context);
                Value::ZeroExtend(ZeroExtend(Intern::new(inner_subst), *size))
            }
            Value::SignExtend(SignExtend(inner, size)) => {
                let inner_subst = inner.as_ref().substitute(context);
                Value::SignExtend(SignExtend(Intern::new(inner_subst), *size))
            }
            Value::Extract(Extract(inner, offset, size)) => {
                let inner_subst = inner.as_ref().substitute(context);
                Value::Extract(Extract(Intern::new(inner_subst), *offset, *size))
            }
            Value::PopCount(PopCount(inner)) => {
                let inner_subst = inner.as_ref().substitute(context);
                Value::PopCount(PopCount(Intern::new(inner_subst)))
            }
            Value::Int2Comp(Int2CompExpr(inner, size)) => {
                let inner_subst = inner.as_ref().substitute(context);
                Value::Int2Comp(Int2CompExpr(Intern::new(inner_subst), *size))
            }

            // Comparison operators: substitute both operands
            Value::IntSLess(IntSLess(a, b)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::IntSLess(IntSLess(Intern::new(a_subst), Intern::new(b_subst)))
            }
            Value::IntEqual(IntEqual(a, b)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::IntEqual(IntEqual(Intern::new(a_subst), Intern::new(b_subst)))
            }
            Value::IntLess(IntLess(a, b)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::IntLess(IntLess(Intern::new(a_subst), Intern::new(b_subst)))
            }
            Value::IntNotEqual(IntNotEqual(a, b)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::IntNotEqual(IntNotEqual(Intern::new(a_subst), Intern::new(b_subst)))
            }
            Value::IntLessEqual(IntLessEqual(a, b)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::IntLessEqual(IntLessEqual(Intern::new(a_subst), Intern::new(b_subst)))
            }
            Value::IntSLessEqual(IntSLessEqual(a, b)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::IntSLessEqual(IntSLessEqual(Intern::new(a_subst), Intern::new(b_subst)))
            }
            Value::IntCarry(IntCarry(a, b)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::IntCarry(IntCarry(Intern::new(a_subst), Intern::new(b_subst)))
            }
            Value::IntSCarry(IntSCarry(a, b)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::IntSCarry(IntSCarry(Intern::new(a_subst), Intern::new(b_subst)))
            }
            Value::IntSBorrow(IntSBorrow(a, b)) => {
                let a_subst = a.as_ref().substitute(context);
                let b_subst = b.as_ref().substitute(context);
                Value::IntSBorrow(IntSBorrow(Intern::new(a_subst), Intern::new(b_subst)))
            }
        };

        // Simplify the result to handle cycles and normalize expressions
        result.simplify()
    }
}

impl Simplify for AddExpr {
    fn simplify(&self) -> Value {
        let a_intern = self.0;
        let b_intern = self.1;

        // simplify children first
        let a_s = a_intern.as_ref().simplify();
        let b_s = b_intern.as_ref().simplify();

        // if any child is Top, the result is Top
        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        // both const -> fold using signed wrapping arithmetic consistent with prior behavior
        if let (Some(a_vn), Some(b_vn)) = (a_s.as_const(), b_s.as_const()) {
            let a = a_vn.offset() as i64;
            let b = b_vn.offset() as i64;
            let res = a.wrapping_add(b);
            let size = Value::derive_size_from(&a_s).max(Value::derive_size_from(&b_s));
            return Value::make_const(res, size as u32);
        }

        // normalization: ensure constants are on the right
        let (left, right) = Value::normalize_commutative(a_s, b_s);

        // expr + 0 -> expr
        // expr + (- |a|) -> expr - a
        if let Some(0) = right.as_const().map(|vn| vn.offset() as i64) {
            return left;
        }

        // ((expr + #a) + #b) -> (expr + #(a + b))
        if let Value::Add(AddExpr(left_inner_left, left_inner_right, _)) = &left {
            if let Some(inner_right_vn) = left_inner_right.as_ref().as_const() {
                if let Some(right_vn) = right.as_const() {
                    let inner_right_const = inner_right_vn.offset() as i64;
                    let right_const = right_vn.offset() as i64;
                    let res = inner_right_const.wrapping_add(right_const);
                    let size = std::cmp::max(
                        left_inner_left.as_ref().size(),
                        Value::derive_size_from(&Value::make_const(res, 8u32)),
                    );
                    let new_const = Value::make_const(res, size as u32);
                    return AddExpr(*left_inner_left, Intern::new(new_const), size).simplify();
                }
            }
        }

        // ((expr - #a) + #b) -> (expr - #(a - b)) or (expr + #(b - a))
        if let Value::Sub(SubExpr(expr, a, _)) = &left {
            if let Some(a_vn) = a.as_ref().as_const() {
                if let Some(b_vn) = right.as_const() {
                    let a_const = a_vn.offset() as i64;
                    let b = b_vn.offset() as i64;
                    let res = a_const.wrapping_sub(b);
                    let size = std::cmp::max(expr.as_ref().size(), Value::derive_size_from(&left));

                    // If res is negative, create Add instead of Sub to avoid infinite loop
                    if res < 0 {
                        let new_const = Value::make_const(-res, size as u32);
                        return AddExpr(*expr, Intern::new(new_const), size).simplify();
                    } else {
                        let new_const = Value::make_const(res, size as u32);
                        return SubExpr(*expr, Intern::new(new_const), size).simplify();
                    }
                }
            }
        }

        // default: rebuild with simplified children; size is max of children
        let s = std::cmp::max(left.size(), right.size());
        Value::Add(AddExpr(Intern::new(left), Intern::new(right), s))
    }
}

impl Simplify for SubExpr {
    fn simplify(&self) -> Value {
        let a_intern = self.0;
        let b_intern = self.1;

        let a_s = a_intern.as_ref().simplify();
        let b_s = b_intern.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        // both const -> fold
        if let (Some(left_vn), Some(right_vn)) = (a_s.as_const(), b_s.as_const()) {
            let left = left_vn.offset() as i64;
            let right = right_vn.offset() as i64;
            let res = left.wrapping_sub(right);
            let size = Value::derive_size_from(&a_s).max(Value::derive_size_from(&b_s));
            return Value::make_const(res, size as u32);
        }

        // DO NOT normalize for subtraction - it is not commutative!
        // Using the simplified children directly preserves the order.
        let left = a_s;
        let right = b_s;

        // expr - 0 -> expr
        // expr - (- |a|) -> expr + a
        match right.as_const().map(|vn| vn.offset() as i64) {
            Some(0) => {
                return left;
            }
            Some(a) => {
                if a < 0 {
                    let new_const = Value::make_const(-a, Value::derive_size_from(&left) as u32);
                    let size = left.size();
                    let add = AddExpr(Intern::new(left), Intern::new(new_const), size).simplify();
                    return add;
                }
            }
            _ => {}
        }

        // x - x -> 0
        if left == right {
            let size = Value::derive_size_from(&left);
            return Value::make_const(0, size as u32);
        }

        // ((expr + #a) - #b) -> (expr + #(a - b)) or (expr - #(b - a))
        if let Value::Add(AddExpr(expr, a, _)) = &left {
            if let Some(a_vn) = a.as_ref().as_const() {
                if let Some(b_vn) = right.as_const() {
                    let a_val = a_vn.offset() as i64;
                    let b_val = b_vn.offset() as i64;
                    let res = a_val.wrapping_sub(b_val);
                    let size = std::cmp::max(expr.as_ref().size(), Value::derive_size_from(&left));

                    // res = a - b (net constant); positive → Add, negative → Sub with -res
                    if res < 0 {
                        let new_const = Value::make_const(-res, size as u32);
                        return SubExpr(*expr, Intern::new(new_const), size).simplify();
                    } else {
                        let new_const = Value::make_const(res, size as u32);
                        return AddExpr(*expr, Intern::new(new_const), size).simplify();
                    }
                }
            }
        }

        // ((expr - #a) - #b) -> (expr - #(a + b))
        if let Value::Sub(SubExpr(expr, a, _)) = &left {
            if let Some(a_vn) = a.as_ref().as_const() {
                if let Some(b_vn) = right.as_const() {
                    let a_val = a_vn.offset() as i64;
                    let b_val = b_vn.offset() as i64;
                    let res = a_val.wrapping_add(b_val);
                    let size = std::cmp::max(expr.as_ref().size(), Value::derive_size_from(&left));
                    let new_const = Value::make_const(res, size as u32);
                    return SubExpr(*expr, Intern::new(new_const), size).simplify();
                }
            }
        }

        let s = std::cmp::max(left.size(), right.size());
        Value::Sub(SubExpr(Intern::new(left), Intern::new(right), s))
    }
}

impl Simplify for MulExpr {
    fn simplify(&self) -> Value {
        let a_intern = self.0;
        let b_intern = self.1;

        let a_s = a_intern.as_ref().simplify();
        let b_s = b_intern.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        // normalization: prefer constant on the right
        let (left, right) = Value::normalize_commutative(a_s, b_s);

        // both const -> fold
        if let (Some(a_vn), Some(b_vn)) = (left.as_const(), right.as_const()) {
            let a_v = a_vn.offset() as i64;
            let b_v = b_vn.offset() as i64;
            let res = a_v.wrapping_mul(b_v);
            let size = Value::derive_size_from(&left).max(Value::derive_size_from(&right));
            return Value::make_const(res, size as u32);
        }

        // expr * 1 -> expr
        if right.as_const().map(|vn| vn.offset() as i64) == Some(1) {
            return left;
        }

        // expr * 0 -> 0
        if right.as_const().map(|vn| vn.offset() as i64) == Some(0) {
            let size = Value::derive_size_from(&left);
            return Value::make_const(0, size as u32);
        }

        let s = std::cmp::max(left.size(), right.size());
        Value::Mul(MulExpr(Intern::new(left), Intern::new(right), s))
    }
}

impl Simplify for Choice {
    fn simplify(&self) -> Value {
        let a_intern = self.0;
        let b_intern = self.1;

        // simplify children first
        let a_s = a_intern.as_ref().simplify();
        let b_s = b_intern.as_ref().simplify();

        // if either child is Top, the result is Top
        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        // normalize so that if one side is a Choice and the other is not, the Choice is on the right
        // (canonical shape: non-Choice on left, Choice on right)
        let (mut left, mut right) = Value::normalize_choice(a_s, b_s);

        // If both sides are non-Choice, enforce deterministic ordering by variant rank.
        if !matches!(left, Value::Choice(_))
            && !matches!(right, Value::Choice(_))
            && Value::variant_rank(&left) > Value::variant_rank(&right)
        {
            std::mem::swap(&mut left, &mut right);
        }

        // identical children => just return one
        if left == right {
            return left;
        }

        // Collapse nested duplicates: Choice(a, Choice(a, b)) -> Choice(a, b)
        if let Value::Choice(Choice(inner_a, inner_b, _)) = &right {
            if inner_a.as_ref() == &left {
                let inner =
                    Value::Choice(Choice(Intern::new(left), *inner_b, right.size())).simplify();
                return inner;
            }
            if inner_b.as_ref() == &left {
                let inner =
                    Value::Choice(Choice(Intern::new(left), *inner_a, right.size())).simplify();
                return inner;
            }
        }

        // Factor common child between two Choices:
        // Choice(Choice(a,b), Choice(a,c)) -> Choice(a, Choice(b,c)) and symmetric variants.
        if let (Value::Choice(Choice(l1, l2, _)), Value::Choice(Choice(r1, r2, _))) =
            (&left, &right)
        {
            // check all combinations for equal common child
            if l1.as_ref() == r1.as_ref() {
                let inner = Value::Choice(Choice(
                    *l2,
                    *r2,
                    std::cmp::max(l2.as_ref().size(), r2.as_ref().size()),
                ))
                .simplify();
                let s = std::cmp::max(l1.as_ref().size(), inner.size());
                return Value::Choice(Choice(*l1, Intern::new(inner), s)).simplify();
            }
            if l1.as_ref() == r2.as_ref() {
                let inner = Value::Choice(Choice(
                    *l2,
                    *r1,
                    std::cmp::max(l2.as_ref().size(), r1.as_ref().size()),
                ))
                .simplify();
                let s = std::cmp::max(l1.as_ref().size(), inner.size());
                return Value::Choice(Choice(*l1, Intern::new(inner), s)).simplify();
            }
            if l2.as_ref() == r1.as_ref() {
                let inner = Value::Choice(Choice(
                    *l1,
                    *r2,
                    std::cmp::max(l1.as_ref().size(), r2.as_ref().size()),
                ))
                .simplify();
                let s = std::cmp::max(l2.as_ref().size(), inner.size());
                return Value::Choice(Choice(*l2, Intern::new(inner), s)).simplify();
            }
            if l2.as_ref() == r2.as_ref() {
                let inner = Value::Choice(Choice(
                    *l1,
                    *r1,
                    std::cmp::max(l1.as_ref().size(), r1.as_ref().size()),
                ))
                .simplify();
                let s = std::cmp::max(l2.as_ref().size(), inner.size());
                return Value::Choice(Choice(*l2, Intern::new(inner), s)).simplify();
            }
        }

        // default: rebuild with simplified children
        let s = std::cmp::max(left.size(), right.size());
        Value::Choice(Choice(Intern::new(left), Intern::new(right), s))
    }
}

impl Simplify for XorExpr {
    fn simplify(&self) -> Value {
        let a_intern = self.0;
        let b_intern = self.1;

        // simplify children first
        let a_s = a_intern.as_ref().simplify();
        let b_s = b_intern.as_ref().simplify();

        // if either child is Top, the result is Top
        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        // normalize: prefer constant on the right
        let (left, right) = Value::normalize_commutative(a_s, b_s);

        // both const -> fold
        if let (Some(left_vn), Some(right_vn)) = (left.as_const(), right.as_const()) {
            let left_val = left_vn.offset();
            let right_val = right_vn.offset();
            let res = (left_val ^ right_val) as i64;
            let size = Value::derive_size_from(&left).max(Value::derive_size_from(&right));
            return Value::make_const(res, size as u32);
        }

        // identical children => 0 (x XOR x = 0)
        if left == right {
            let size = Value::derive_size_from(&left);
            return Value::make_const(0, size as u32);
        }

        // expr XOR 0 -> expr
        if right.as_const().map(|vn| vn.offset()) == Some(0) {
            return left;
        }

        // default: rebuild with simplified children
        let s = std::cmp::max(left.size(), right.size());
        Value::Xor(XorExpr(Intern::new(left), Intern::new(right), s))
    }
}

impl Simplify for AndExpr {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        let (left, right) = Value::normalize_commutative(a_s, b_s);

        // both const -> fold
        if let (Some(left_vn), Some(right_vn)) = (left.as_const(), right.as_const()) {
            let res = (left_vn.offset() & right_vn.offset()) as i64;
            let size = Value::derive_size_from(&left).max(Value::derive_size_from(&right));
            return Value::make_const(res, size as u32);
        }

        // x & x -> x
        if left == right {
            return left;
        }

        // x & 0 -> 0
        if right.as_const().map(|vn| vn.offset()) == Some(0) {
            let size = Value::derive_size_from(&left);
            return Value::make_const(0, size as u32);
        }

        // x & all-ones -> x
        let all_ones = match left.size() {
            1 => Some(0xFF_u64),
            2 => Some(0xFFFF_u64),
            4 => Some(0xFFFF_FFFF_u64),
            8 => Some(u64::MAX),
            _ => None,
        };
        if let Some(mask) = all_ones {
            if right.as_const().map(|vn| vn.offset()) == Some(mask) {
                return left;
            }
        }

        let s = std::cmp::max(left.size(), right.size());
        Value::And(AndExpr(Intern::new(left), Intern::new(right), s))
    }
}

impl Simplify for OrExpr {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        let (left, right) = Value::normalize_commutative(a_s, b_s);

        // both const -> fold
        if let (Some(left_vn), Some(right_vn)) = (left.as_const(), right.as_const()) {
            let res = (left_vn.offset() | right_vn.offset()) as i64;
            let size = Value::derive_size_from(&left).max(Value::derive_size_from(&right));
            return Value::make_const(res, size as u32);
        }

        // x | x -> x
        if left == right {
            return left;
        }

        // x | 0 -> x
        if right.as_const().map(|vn| vn.offset()) == Some(0) {
            return left;
        }

        // x | all-ones -> all-ones
        let all_ones = match left.size() {
            1 => Some(0xFF_u64),
            2 => Some(0xFFFF_u64),
            4 => Some(0xFFFF_FFFF_u64),
            8 => Some(u64::MAX),
            _ => None,
        };
        if let Some(mask) = all_ones {
            if right.as_const().map(|vn| vn.offset()) == Some(mask) {
                let size = Value::derive_size_from(&left);
                return Value::make_const(mask as i64, size as u32);
            }
        }

        let s = std::cmp::max(left.size(), right.size());
        Value::Or(OrExpr(Intern::new(left), Intern::new(right), s))
    }
}

impl Simplify for IntLeftShiftExpr {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        // both const -> fold
        if let (Some(left_vn), Some(right_vn)) = (a_s.as_const(), b_s.as_const()) {
            let left_val = left_vn.offset();
            let shift_amt = right_vn.offset();
            let size_bits = (left_vn.size() * 8) as u32;

            // If shift amount is >= bit width, result is 0
            if shift_amt >= size_bits as u64 {
                let size = Value::derive_size_from(&a_s);
                return Value::make_const(0, size as u32);
            }

            let result = left_val.wrapping_shl(shift_amt as u32);
            let masked = result & mask_for_size(left_vn.size());
            let size = Value::derive_size_from(&a_s);
            return Value::make_const(masked as i64, size as u32);
        }

        // expr << 0 -> expr
        if b_s.as_const().map(|vn| vn.offset()) == Some(0) {
            return a_s;
        }

        let s = std::cmp::max(a_s.size(), b_s.size());
        Value::IntLeftShift(IntLeftShiftExpr(Intern::new(a_s), Intern::new(b_s), s))
    }
}

impl Simplify for IntRightShiftExpr {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        // both const -> fold (unsigned/logical right shift)
        if let (Some(left_vn), Some(right_vn)) = (a_s.as_const(), b_s.as_const()) {
            let left_val = left_vn.offset();
            let shift_amt = right_vn.offset();
            let size_bits = (left_vn.size() * 8) as u32;

            // If shift amount is >= bit width, result is 0
            if shift_amt >= size_bits as u64 {
                let size = Value::derive_size_from(&a_s);
                return Value::make_const(0, size as u32);
            }

            let result = left_val.wrapping_shr(shift_amt as u32);
            let size = Value::derive_size_from(&a_s);
            return Value::make_const(result as i64, size as u32);
        }

        // expr >> 0 -> expr
        if b_s.as_const().map(|vn| vn.offset()) == Some(0) {
            return a_s;
        }

        let s = std::cmp::max(a_s.size(), b_s.size());
        Value::IntRightShift(IntRightShiftExpr(Intern::new(a_s), Intern::new(b_s), s))
    }
}

impl Simplify for IntSignedRightShiftExpr {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        // both const -> fold (signed/arithmetic right shift)
        if let (Some(left_vn), Some(right_vn)) = (a_s.as_const(), b_s.as_const()) {
            let left_val = left_vn.offset();
            let shift_amt = right_vn.offset();
            let size_bits = (left_vn.size() * 8) as u32;

            // Convert to signed value for arithmetic shift
            let signed_val = if size_bits < 64 {
                let sign_bit = 1u64 << (size_bits - 1);
                if left_val & sign_bit != 0 {
                    // Negative: sign-extend to i64
                    (left_val | (u64::MAX << size_bits)) as i64
                } else {
                    left_val as i64
                }
            } else {
                left_val as i64
            };

            // If shift amount is >= bit width, result is all sign bits (0 or -1)
            if shift_amt >= size_bits as u64 {
                let result = if signed_val < 0 { -1i64 } else { 0i64 };
                let size = Value::derive_size_from(&a_s);
                return Value::make_const(result, size as u32);
            }

            let result = signed_val.wrapping_shr(shift_amt as u32);
            let masked = (result as u64) & mask_for_size(left_vn.size());
            let size = Value::derive_size_from(&a_s);
            return Value::make_const(masked as i64, size as u32);
        }

        // expr s>> 0 -> expr
        if b_s.as_const().map(|vn| vn.offset()) == Some(0) {
            return a_s;
        }

        let s = std::cmp::max(a_s.size(), b_s.size());
        Value::IntSignedRightShift(IntSignedRightShiftExpr(
            Intern::new(a_s),
            Intern::new(b_s),
            s,
        ))
    }
}

impl Simplify for Load {
    fn simplify(&self) -> Value {
        let a_intern = self.0;
        let a_s = a_intern.as_ref().simplify();

        if matches!(a_s, Value::Top) {
            return Value::Top;
        }

        // keep the same size as recorded on this Load node
        Value::Load(Load(Intern::new(a_s), self.1))
    }
}

/// Return a bitmask covering exactly `size_bytes` bytes (up to 8).
fn mask_for_size(size_bytes: usize) -> u64 {
    if size_bytes >= 8 {
        u64::MAX
    } else {
        (1u64 << (size_bytes * 8)).wrapping_sub(1)
    }
}

impl Simplify for ZeroExtend {
    fn simplify(&self) -> Value {
        let ZeroExtend(inner_intern, output_size) = self;
        let inner = inner_intern.as_ref().simplify();

        if matches!(inner, Value::Top) {
            return Value::Top;
        }

        // identity: extending to the same size is a no-op
        if inner.size() == *output_size {
            return inner;
        }

        // constant folding: mask to source size (unsigned), then store in output size
        if let Some(vn) = inner.as_const() {
            let src_value = vn.offset() & mask_for_size(vn.size());
            return Value::make_const(src_value as i64, *output_size as u32);
        }

        // chain: zext(zext(x, s1), s2) where s2 >= s1 → zext(x, s2)
        if let Value::ZeroExtend(ZeroExtend(inner2, s1)) = &inner {
            if *output_size >= *s1 {
                return ZeroExtend(*inner2, *output_size).simplify();
            }
        }

        Value::ZeroExtend(ZeroExtend(Intern::new(inner), *output_size))
    }
}

impl Simplify for SignExtend {
    fn simplify(&self) -> Value {
        let SignExtend(inner_intern, output_size) = self;
        let inner = inner_intern.as_ref().simplify();

        if matches!(inner, Value::Top) {
            return Value::Top;
        }

        // identity: extending to the same size is a no-op
        if inner.size() == *output_size {
            return inner;
        }

        // constant folding
        if let Some(vn) = inner.as_const() {
            let src_size = vn.size();
            let raw = vn.offset();
            // sign-extend from src_size bytes to u64
            let sign_extended = if src_size > 0 && src_size < 8 {
                let sign_bit = 1u64 << (src_size * 8 - 1);
                if raw & sign_bit != 0 {
                    raw | (u64::MAX << (src_size * 8))
                } else {
                    raw
                }
            } else {
                raw
            };
            let masked = sign_extended & mask_for_size(*output_size);
            return Value::make_const(masked as i64, *output_size as u32);
        }

        // chain: sext(sext(x, s1), s2) where s2 >= s1 → sext(x, s2)
        if let Value::SignExtend(SignExtend(inner2, s1)) = &inner {
            if *output_size >= *s1 {
                return SignExtend(*inner2, *output_size).simplify();
            }
        }

        Value::SignExtend(SignExtend(Intern::new(inner), *output_size))
    }
}

impl Simplify for Extract {
    fn simplify(&self) -> Value {
        let Extract(inner_intern, byte_offset, output_size) = self;
        let inner = inner_intern.as_ref().simplify();

        if matches!(inner, Value::Top) {
            return Value::Top;
        }

        // identity: extracting the full value at offset 0 is a no-op
        if *byte_offset == 0 && inner.size() == *output_size {
            return inner;
        }

        // constant folding
        if let Some(vn) = inner.as_const() {
            let shifted = vn.offset() >> (byte_offset * 8);
            let masked = shifted & mask_for_size(*output_size);
            return Value::make_const(masked as i64, *output_size as u32);
        }

        Value::Extract(Extract(Intern::new(inner), *byte_offset, *output_size))
    }
}

impl Simplify for IntEqual {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        if let (Some(a_vn), Some(b_vn)) = (a_s.as_const(), b_s.as_const()) {
            let result = (a_vn.offset() == b_vn.offset()) as i64;
            return Value::make_const(result, 1);
        }

        if a_s == b_s {
            return Value::make_const(1, 1);
        }

        Value::IntEqual(IntEqual(Intern::new(a_s), Intern::new(b_s)))
    }
}

impl Simplify for IntLess {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        if let (Some(a_vn), Some(b_vn)) = (a_s.as_const(), b_s.as_const()) {
            let result = (a_vn.offset() < b_vn.offset()) as i64;
            return Value::make_const(result, 1);
        }

        if a_s == b_s {
            return Value::make_const(0, 1);
        }

        Value::IntLess(IntLess(Intern::new(a_s), Intern::new(b_s)))
    }
}

impl Simplify for IntSLess {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        if let (Some(a_vn), Some(b_vn)) = (a_s.as_const(), b_s.as_const()) {
            let result = ((a_vn.offset() as i64) < (b_vn.offset() as i64)) as i64;
            return Value::make_const(result, 1);
        }

        if a_s == b_s {
            return Value::make_const(0, 1);
        }

        Value::IntSLess(IntSLess(Intern::new(a_s), Intern::new(b_s)))
    }
}

impl Simplify for PopCount {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();

        if matches!(a_s, Value::Top) {
            return Value::Top;
        }

        if let Some(vn) = a_s.as_const() {
            let result = vn.offset().count_ones() as i64;
            return Value::make_const(result, 1);
        }

        Value::PopCount(PopCount(Intern::new(a_s)))
    }
}

impl Simplify for IntNotEqual {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        if let (Some(a_vn), Some(b_vn)) = (a_s.as_const(), b_s.as_const()) {
            let result = (a_vn.offset() != b_vn.offset()) as i64;
            return Value::make_const(result, 1);
        }

        if a_s == b_s {
            return Value::make_const(0, 1);
        }

        Value::IntNotEqual(IntNotEqual(Intern::new(a_s), Intern::new(b_s)))
    }
}

impl Simplify for IntLessEqual {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        if let (Some(a_vn), Some(b_vn)) = (a_s.as_const(), b_s.as_const()) {
            let result = (a_vn.offset() <= b_vn.offset()) as i64;
            return Value::make_const(result, 1);
        }

        if a_s == b_s {
            return Value::make_const(1, 1);
        }

        Value::IntLessEqual(IntLessEqual(Intern::new(a_s), Intern::new(b_s)))
    }
}

impl Simplify for IntSLessEqual {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        if let (Some(a_vn), Some(b_vn)) = (a_s.as_const(), b_s.as_const()) {
            let result = ((a_vn.offset() as i64) <= (b_vn.offset() as i64)) as i64;
            return Value::make_const(result, 1);
        }

        if a_s == b_s {
            return Value::make_const(1, 1);
        }

        Value::IntSLessEqual(IntSLessEqual(Intern::new(a_s), Intern::new(b_s)))
    }
}

impl Simplify for IntCarry {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        if let (Some(a_vn), Some(b_vn)) = (a_s.as_const(), b_s.as_const()) {
            let bits = (a_vn.size() * 8) as u32;
            let carry = (a_vn.offset() as u128 + b_vn.offset() as u128) >> bits;
            return Value::make_const((carry != 0) as i64, 1);
        }

        Value::IntCarry(IntCarry(Intern::new(a_s), Intern::new(b_s)))
    }
}

impl Simplify for IntSCarry {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        if let (Some(a_vn), Some(b_vn)) = (a_s.as_const(), b_s.as_const()) {
            let n = a_vn.size() * 8;
            let mask = if n == 64 {
                u64::MAX
            } else {
                (1u64 << n).wrapping_sub(1)
            };
            let sign_mask = 1u64 << (n - 1);
            let a_val = a_vn.offset() & mask;
            let b_val = b_vn.offset() & mask;
            let sum = a_val.wrapping_add(b_val) & mask;
            let overflow = ((a_val ^ sum) & (b_val ^ sum) & sign_mask) != 0;
            return Value::make_const(overflow as i64, 1);
        }

        Value::IntSCarry(IntSCarry(Intern::new(a_s), Intern::new(b_s)))
    }
}

impl Simplify for IntSBorrow {
    fn simplify(&self) -> Value {
        let a_s = self.0.as_ref().simplify();
        let b_s = self.1.as_ref().simplify();

        if matches!(a_s, Value::Top) || matches!(b_s, Value::Top) {
            return Value::Top;
        }

        if let (Some(a_vn), Some(b_vn)) = (a_s.as_const(), b_s.as_const()) {
            let n = a_vn.size() * 8;
            let mask = if n == 64 {
                u64::MAX
            } else {
                (1u64 << n).wrapping_sub(1)
            };
            let sign_mask = 1u64 << (n - 1);
            let a_val = a_vn.offset() & mask;
            let b_val = b_vn.offset() & mask;
            let diff = a_val.wrapping_sub(b_val) & mask;
            let overflow = ((a_val ^ b_val) & (a_val ^ diff) & sign_mask) != 0;
            return Value::make_const(overflow as i64, 1);
        }

        if a_s == b_s {
            return Value::make_const(0, 1);
        }

        Value::IntSBorrow(IntSBorrow(Intern::new(a_s), Intern::new(b_s)))
    }
}

impl Simplify for Int2CompExpr {
    fn simplify(&self) -> Value {
        let Int2CompExpr(inner_intern, output_size) = self;
        let inner = inner_intern.as_ref().simplify();

        if matches!(inner, Value::Top) {
            return Value::Top;
        }

        // constant folding: compute two's complement = -x
        if let Some(vn) = inner.as_const() {
            let value = vn.offset() as i64;
            let negated = value.wrapping_neg();
            return Value::make_const(negated, *output_size as u32);
        }

        // identity: int_2comp(int_2comp(x)) = x
        if let Value::Int2Comp(Int2CompExpr(inner2, _)) = &inner {
            return inner2.as_ref().clone();
        }

        Value::Int2Comp(Int2CompExpr(Intern::new(inner), *output_size))
    }
}

fn fmt_operand_jingle(f: &mut Formatter<'_>, v: &Value, info: &SleighArchInfo) -> std::fmt::Result {
    if v.is_compound() {
        write!(f, "(")?;
        v.fmt_jingle(f, info)?;
        write!(f, ")")
    } else {
        v.fmt_jingle(f, info)
    }
}

fn fmt_operand(f: &mut std::fmt::Formatter<'_>, v: &Value) -> std::fmt::Result {
    if v.is_compound() {
        write!(f, "({v})")
    } else {
        write!(f, "{v}")
    }
}

fn fmt_operand_hex(f: &mut std::fmt::Formatter<'_>, v: &Value) -> std::fmt::Result {
    if v.is_compound() {
        write!(f, "({v:x})")
    } else {
        write!(f, "{v:x}")
    }
}

impl JingleDisplay for Value {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        match self {
            Value::Entry(Entry(vn)) => write!(f, "{}", vn.display(info)),
            Value::Const(vn) => {
                // print constant offset in hex (retain prior appearance)
                write!(f, "{:#x}", vn.as_ref().offset())
            }
            Value::Offset(Offset(vn, con)) => {
                write!(
                    f,
                    "offset({},{})",
                    vn.as_ref().0.display(info),
                    con.as_ref().0.display(info)
                )
            }
            Value::Mul(MulExpr(a, b, _)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "*")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::Add(AddExpr(a, b, _)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "+")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::Sub(SubExpr(a, b, _)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "-")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::Choice(Choice(a, b, _)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "||")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::Xor(XorExpr(a, b, _)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "^")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::Or(OrExpr(a, b, _)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "|")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::And(AndExpr(a, b, _)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "&")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::IntLeftShift(IntLeftShiftExpr(a, b, _)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "<<")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::IntRightShift(IntRightShiftExpr(a, b, _)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, ">>")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::IntSignedRightShift(IntSignedRightShiftExpr(a, b, _)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "s>>")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::Load(Load(a, _)) => write!(f, "Load({})", a.as_ref().display(info)),
            Value::ZeroExtend(ZeroExtend(a, s)) => {
                write!(f, "zext(")?;
                a.as_ref().fmt_jingle(f, info)?;
                write!(f, ", {s})")
            }
            Value::SignExtend(SignExtend(a, s)) => {
                write!(f, "sext(")?;
                a.as_ref().fmt_jingle(f, info)?;
                write!(f, ", {s})")
            }
            Value::Extract(Extract(a, off, s)) => {
                write!(f, "extract(")?;
                a.as_ref().fmt_jingle(f, info)?;
                write!(f, ", {off}:{s})")
            }
            Value::IntEqual(IntEqual(a, b)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "==")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::IntSLess(IntSLess(a, b)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "s<")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::IntLess(IntLess(a, b)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "u<")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::PopCount(PopCount(a)) => {
                write!(f, "popcount(")?;
                a.as_ref().fmt_jingle(f, info)?;
                write!(f, ")")
            }
            Value::Int2Comp(Int2CompExpr(a, _)) => {
                write!(f, "int_2comp(")?;
                a.as_ref().fmt_jingle(f, info)?;
                write!(f, ")")
            }
            Value::IntNotEqual(IntNotEqual(a, b)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "!=")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::IntLessEqual(IntLessEqual(a, b)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "u<=")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::IntSLessEqual(IntSLessEqual(a, b)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "s<=")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::IntCarry(IntCarry(a, b)) => {
                write!(f, "carry(")?;
                a.as_ref().fmt_jingle(f, info)?;
                write!(f, ",")?;
                b.as_ref().fmt_jingle(f, info)?;
                write!(f, ")")
            }
            Value::IntSCarry(IntSCarry(a, b)) => {
                write!(f, "scarry(")?;
                a.as_ref().fmt_jingle(f, info)?;
                write!(f, ",")?;
                b.as_ref().fmt_jingle(f, info)?;
                write!(f, ")")
            }
            Value::IntSBorrow(IntSBorrow(a, b)) => {
                write!(f, "sborrow(")?;
                a.as_ref().fmt_jingle(f, info)?;
                write!(f, ",")?;
                b.as_ref().fmt_jingle(f, info)?;
                write!(f, ")")
            }
            Value::Top => write!(f, "⊤"),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Entry(Entry(vn)) => {
                // Delegate to VarNode's Display implementation
                write!(f, "{}", vn)
            }
            Value::Const(vn) => {
                // Print constant offset in hex (consistent with jingle display)
                write!(f, "{:#x}", vn.as_ref().offset())
            }
            Value::Offset(Offset(vn, off)) => {
                write!(f, "offset({}, {})", vn.0, off.0)
            }
            Value::Mul(MulExpr(a, b, _)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "*")?;
                fmt_operand(f, b.as_ref())
            }
            Value::Add(AddExpr(a, b, _)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "+")?;
                fmt_operand(f, b.as_ref())
            }
            Value::Sub(SubExpr(a, b, _)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "-")?;
                fmt_operand(f, b.as_ref())
            }
            Value::Choice(Choice(a, b, _)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "||")?;
                fmt_operand(f, b.as_ref())
            }
            Value::Xor(XorExpr(a, b, _)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "^")?;
                fmt_operand(f, b.as_ref())
            }
            Value::Or(OrExpr(a, b, _)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "|")?;
                fmt_operand(f, b.as_ref())
            }
            Value::And(AndExpr(a, b, _)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "&")?;
                fmt_operand(f, b.as_ref())
            }
            Value::IntLeftShift(IntLeftShiftExpr(a, b, _)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "<<")?;
                fmt_operand(f, b.as_ref())
            }
            Value::IntRightShift(IntRightShiftExpr(a, b, _)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, ">>")?;
                fmt_operand(f, b.as_ref())
            }
            Value::IntSignedRightShift(IntSignedRightShiftExpr(a, b, _)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "s>>")?;
                fmt_operand(f, b.as_ref())
            }
            Value::Load(Load(a, _)) => {
                // Load(child)
                write!(f, "Load({})", a.as_ref())
            }
            Value::ZeroExtend(ZeroExtend(a, s)) => write!(f, "zext({}, {s})", a.as_ref()),
            Value::SignExtend(SignExtend(a, s)) => write!(f, "sext({}, {s})", a.as_ref()),
            Value::Extract(Extract(a, off, s)) => {
                write!(f, "extract({}, {off}:{s})", a.as_ref())
            }
            Value::IntEqual(IntEqual(a, b)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "==")?;
                fmt_operand(f, b.as_ref())
            }
            Value::IntSLess(IntSLess(a, b)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "s<")?;
                fmt_operand(f, b.as_ref())
            }
            Value::IntLess(IntLess(a, b)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "u<")?;
                fmt_operand(f, b.as_ref())
            }
            Value::PopCount(PopCount(a)) => {
                write!(f, "popcount({})", a.as_ref())
            }
            Value::Int2Comp(Int2CompExpr(a, _)) => {
                write!(f, "int_2comp({})", a.as_ref())
            }
            Value::IntNotEqual(IntNotEqual(a, b)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "!=")?;
                fmt_operand(f, b.as_ref())
            }
            Value::IntLessEqual(IntLessEqual(a, b)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "u<=")?;
                fmt_operand(f, b.as_ref())
            }
            Value::IntSLessEqual(IntSLessEqual(a, b)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "s<=")?;
                fmt_operand(f, b.as_ref())
            }
            Value::IntCarry(IntCarry(a, b)) => {
                write!(f, "carry({}, {})", a.as_ref(), b.as_ref())
            }
            Value::IntSCarry(IntSCarry(a, b)) => {
                write!(f, "scarry({}, {})", a.as_ref(), b.as_ref())
            }
            Value::IntSBorrow(IntSBorrow(a, b)) => {
                write!(f, "sborrow({}, {})", a.as_ref(), b.as_ref())
            }
            Value::Top => {
                // Special top symbol
                write!(f, "⊤")
            }
        }
    }
}

impl std::fmt::LowerHex for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Entry(Entry(vn)) => {
                // VarNode doesn't implement LowerHex; fall back to Display
                write!(f, "{}", vn)
            }
            Value::Const(vn) => {
                // Lower-hex for constants: no 0x prefix, lowercase hex digits
                write!(f, "{:x}", vn.as_ref().offset())
            }
            Value::Offset(Offset(vn, off)) => {
                write!(f, "offset({:x}, {:x})", vn.0, off.0)
            }
            Value::Mul(MulExpr(a, b, _)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "*")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::Add(AddExpr(a, b, _)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "+")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::Sub(SubExpr(a, b, _)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "-")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::Choice(Choice(a, b, _)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "||")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::Xor(XorExpr(a, b, _)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "^")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::Or(OrExpr(a, b, _)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "|")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::And(AndExpr(a, b, _)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "&")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::IntLeftShift(IntLeftShiftExpr(a, b, _)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "<<")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::IntRightShift(IntRightShiftExpr(a, b, _)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, ">>")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::IntSignedRightShift(IntSignedRightShiftExpr(a, b, _)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "s>>")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::Load(Load(a, _)) => {
                write!(f, "Load({:x})", a.as_ref())
            }
            Value::ZeroExtend(ZeroExtend(a, s)) => {
                write!(f, "zext({:x}, {s})", a.as_ref())
            }
            Value::SignExtend(SignExtend(a, s)) => {
                write!(f, "sext({:x}, {s})", a.as_ref())
            }
            Value::Extract(Extract(a, off, s)) => {
                write!(f, "extract({:x}, {off}:{s})", a.as_ref())
            }
            Value::IntEqual(IntEqual(a, b)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "==")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::IntSLess(IntSLess(a, b)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "s<")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::IntLess(IntLess(a, b)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "u<")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::PopCount(PopCount(a)) => {
                write!(f, "popcount({:x})", a.as_ref())
            }
            Value::Int2Comp(Int2CompExpr(a, _)) => {
                write!(f, "int_2comp({:x})", a.as_ref())
            }
            Value::IntNotEqual(IntNotEqual(a, b)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "!=")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::IntLessEqual(IntLessEqual(a, b)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "u<=")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::IntSLessEqual(IntSLessEqual(a, b)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "s<=")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::IntCarry(IntCarry(a, b)) => {
                write!(f, "carry({:x}, {:x})", a.as_ref(), b.as_ref())
            }
            Value::IntSCarry(IntSCarry(a, b)) => {
                write!(f, "scarry({:x}, {:x})", a.as_ref(), b.as_ref())
            }
            Value::IntSBorrow(IntSBorrow(a, b)) => {
                write!(f, "sborrow({:x}, {:x})", a.as_ref(), b.as_ref())
            }
            Value::Top => write!(f, "⊤"),
        }
    }
}

impl Value {
    /// Resolve a VarNode to an existing valuation in the state's direct writes,
    /// to a Const if the VarNode is a constant, or to an Entry if unseen.
    pub fn from_varnode_or_entry(state: &ValuationState, vn: &VarNode) -> Self {
        if vn.is_const() {
            // preserve the size of the incoming varnode
            Value::const_from_varnode(*vn)
        } else if let Some(v) = state.valuation.direct_writes.get(vn) {
            v.clone()
        } else {
            Value::Entry(Entry(*vn))
        }
    }
}

impl JoinSemiLattice for Value {
    fn join(&mut self, _other: &Self) {}
}

#[cfg(test)]
#[path = "value_tests.rs"]
mod value_tests;
