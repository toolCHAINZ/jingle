use crate::{
    analysis::cpa::lattice::JoinSemiLattice,
    display::JingleDisplay,
};
use jingle_sleigh::{SleighArchInfo, VarNode};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use std::{
    borrow::Borrow,
    ops::{BitAnd, BitXor, Deref},
    sync::atomic::{AtomicU64, Ordering},
};
use std::{
    fmt::Formatter,
    ops::{Add, Mul, Sub},
};

/// Global atomic counter used exclusively by [`Value::fresh_unique`] to mint
/// monotonically-increasing, globally-unique identifiers.
static BIND_COUNTER: AtomicU64 = AtomicU64::new(0);

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Value {}
    impl Sealed for std::rc::Rc<super::Value> {}
    impl Sealed for &super::Value {}
    impl Sealed for &std::rc::Rc<super::Value> {}
}

/// Anything that can be used as an operand to a `Value` constructor.
///
/// Implemented for:
/// - `Value` — takes ownership and wraps in `Rc`
/// - `Rc<Value>` — already wrapped; identity conversion (cheap clone)
/// - `&Value` — clones and wraps in `Rc`
/// - `&Rc<Value>` — increments reference count
///
/// This trait is sealed; external implementations are not supported.
pub trait IntoRcValue: sealed::Sealed {
    fn into_rc(self) -> Rc<Value>;
}

impl IntoRcValue for Value {
    fn into_rc(self) -> Rc<Value> {
        Rc::new(self)
    }
}

impl IntoRcValue for Rc<Value> {
    fn into_rc(self) -> Rc<Value> {
        self
    }
}

impl IntoRcValue for &Rc<Value> {
    fn into_rc(self) -> Rc<Value> {
        Rc::clone(self)
    }
}

trait Simplify {
    /// Simplify a Value ast with standard methods like constant folding
    ///
    /// Returns `Rc::clone(outer)` when the node is unchanged, avoiding new
    /// heap allocations for already-simplified subtrees.
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value>;
}

/// An entry value of a direct location
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Entry(VarNode);

impl Deref for Entry {
    type Target = VarNode;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A constant value
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Offset(Entry, Const);

impl Offset {
    pub fn new(base: impl Borrow<Entry>, offset: impl Borrow<Const>) -> Self {
        Self(*base.borrow(), *offset.borrow())
    }

    pub fn base_vn(&self) -> &Entry {
        &self.0
    }

    pub fn offset(&self) -> &Const {
        &self.1
    }

    pub fn overlaps(&self, other: &Self) -> bool {
        // Two offsets overlap if they refer to the same base and their offset ranges intersect.
        if self.base_vn() != other.base_vn() {
            return false;
        }
        let self_start = self.offset().as_ref().offset() as i64;
        let self_end = self_start + self.offset().as_ref().size() as i64;
        let other_start = other.offset().as_ref().offset() as i64;
        let other_end = other_start + other.offset().as_ref().size() as i64;

        // Check if the ranges [self_start, self_end) and [other_start, other_end) overlap
        !(self_end <= other_start || other_end <= self_start)
    }
}

/// A multiplication expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MulExpr(pub Rc<Value>, pub Rc<Value>, pub usize);

/// An addition expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AddExpr(pub Rc<Value>, pub Rc<Value>, pub usize);

/// A subtraction expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SubExpr(pub Rc<Value>, pub Rc<Value>, pub usize);

/// An expression representing two possible values (abstract interpretation choice)
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Choice(pub Rc<Value>, pub Rc<Value>, pub usize);

/// A bitwise XOR expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct XorExpr(pub Rc<Value>, pub Rc<Value>, pub usize);

/// A bitwise OR expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OrExpr(pub Rc<Value>, pub Rc<Value>, pub usize);

/// A bitwise AND expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AndExpr(pub Rc<Value>, pub Rc<Value>, pub usize);

/// A boolean negate expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BoolNegateExpr(pub Rc<Value>);

/// A boolean AND expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BoolAndExpr(pub Rc<Value>, pub Rc<Value>);

/// A boolean OR expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BoolOrExpr(pub Rc<Value>, pub Rc<Value>);

/// A boolean XOR expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BoolXorExpr(pub Rc<Value>, pub Rc<Value>);

/// A left shift expression
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntLeftShiftExpr(pub Rc<Value>, pub Rc<Value>, pub usize);

/// An unsigned right shift expression (logical shift)
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntRightShiftExpr(pub Rc<Value>, pub Rc<Value>, pub usize);

/// A signed right shift expression (arithmetic shift)
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntSignedRightShiftExpr(pub Rc<Value>, pub Rc<Value>, pub usize);

/// A signed comparison operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntSLess(pub Rc<Value>, pub Rc<Value>);

/// An equality comparison operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntEqual(pub Rc<Value>, pub Rc<Value>);

/// An unsigned comparison operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntLess(pub Rc<Value>, pub Rc<Value>);

/// A PopCount operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PopCount(pub Rc<Value>);

/// A two's complement operator (INT_2COMP): computes -x = ~x + 1
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Int2CompExpr(pub Rc<Value>, pub usize);

/// An inequality comparison operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntNotEqual(pub Rc<Value>, pub Rc<Value>);

/// An unsigned less-than-or-equal comparison operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntLessEqual(pub Rc<Value>, pub Rc<Value>);

/// A signed less-than-or-equal comparison operator
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntSLessEqual(pub Rc<Value>, pub Rc<Value>);

/// Unsigned addition carry-out
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntCarry(pub Rc<Value>, pub Rc<Value>);

/// Signed addition overflow (SCARRY)
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntSCarry(pub Rc<Value>, pub Rc<Value>);

/// Signed subtraction overflow (SBORROW)
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IntSBorrow(pub Rc<Value>, pub Rc<Value>);

/// A load of a certain size from a pointer with a certain value
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Load(pub Rc<Value>, pub usize, pub u8);

/// A zero-extension of the inner value to `output_size` bytes
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ZeroExtend(pub Rc<Value>, pub usize);

/// A sign-extension of the inner value to `output_size` bytes
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SignExtend(pub Rc<Value>, pub usize);

/// Extraction of `output_size` bytes from the inner value starting at `byte_offset`
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Extract(pub Rc<Value>, pub usize, pub usize);

/// A globally-unique opaque value with an associated size in bytes.
///
/// The only way to create a fresh `Unique` (other than cloning an existing one)
/// is via [`Value::fresh_unique`], which draws a monotonically-increasing id
/// from a shared atomic counter.
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Bind(u64, usize);

impl Bind {
    /// The globally-unique numeric identifier for this value.
    pub fn id(&self) -> u64 {
        self.0
    }

    /// The size in bytes associated with this unique value.
    pub fn size(&self) -> usize {
        self.1
    }
}

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

impl From<VarNode> for Entry {
    fn from(vn: VarNode) -> Self {
        Self(vn)
    }
}

impl Bind {
    pub fn from_id_size(id: u64, size: usize) -> Self {
        Self(id, size)
    }
}

/// Symbolic valuation built from varnodes and constants (constants are interned VarNodes).
#[derive(Debug, Clone, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
    BoolNegate(BoolNegateExpr),
    BoolAnd(BoolAndExpr),
    BoolOr(BoolOrExpr),
    BoolXor(BoolXorExpr),
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

    /// A globally-unique opaque value. Construct exclusively via
    /// [`Value::fresh_unique`]; cloning an existing `Unique` preserves the id.
    Bind(Bind),

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

    pub fn as_zext(&self) -> Option<&ZeroExtend> {
        match self {
            Value::ZeroExtend(vn_intern) => Some(vn_intern),
            _ => None,
        }
    }

    pub fn as_sext(&self) -> Option<&SignExtend> {
        match self {
            Value::SignExtend(vn_intern) => Some(vn_intern),
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

    /// Accessor for the `Unique` variant.
    pub fn as_bind(&self) -> Option<&Bind> {
        match self {
            Value::Bind(u) => Some(u),
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
                | Value::BoolNegate(_)
                | Value::BoolAnd(_)
                | Value::BoolOr(_)
                | Value::BoolXor(_)
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

    /// Accessor for `BoolNegate` variant.
    pub fn as_bool_negate(&self) -> Option<&BoolNegateExpr> {
        match self {
            Value::BoolNegate(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `BoolAnd` variant.
    pub fn as_bool_and(&self) -> Option<&BoolAndExpr> {
        match self {
            Value::BoolAnd(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `BoolOr` variant.
    pub fn as_bool_or(&self) -> Option<&BoolOrExpr> {
        match self {
            Value::BoolOr(v) => Some(v),
            _ => None,
        }
    }

    /// Accessor for `BoolXor` variant.
    pub fn as_bool_xor(&self) -> Option<&BoolXorExpr> {
        match self {
            Value::BoolXor(v) => Some(v),
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
            Value::Offset(Offset(_, vn)) => vn.0.size(),
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
            Value::BoolNegate(_) | Value::BoolAnd(_) | Value::BoolOr(_) | Value::BoolXor(_) => 1,
            Value::Load(Load(_, s, _)) => *s,
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
            Value::Bind(u) => u.size(),
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
        Value::Offset(Offset(Entry(vn), Const(offset)))
    }

    /// Construct a `Const(...)` from a raw i64 value.
    /// We create a `VarNode` in the constant space with a default size of 8 bytes
    /// (64-bit) unless callers use `make_const` to specify a size explicitly.
    pub fn const_(v: i64, size: usize) -> Self {
        // default to 8-byte sized constant
        let vn = VarNode::new_const(v as u64, size as u32);
        Value::Const(Const(vn))
    }

    /// Construct a `Const(...)` directly from a `VarNode` (already contains size).
    pub fn const_from_varnode(vn: VarNode) -> Self {
        Value::Const(Const(vn))
    }

    /// Construct a fresh [`Value::Bind`] with the given size in bytes.
    ///
    /// Each call draws the next value from a shared global [`AtomicU64`] counter,
    /// so the returned id is guaranteed to be unique within the process lifetime.
    /// The only other way to obtain a `Value::Bind` is by cloning an existing one,
    /// which preserves the original id.
    pub fn fresh_bind(size: usize) -> Self {
        let id = BIND_COUNTER.fetch_add(1, Ordering::Relaxed);
        Value::Bind(Bind(id, size))
    }

    /// Construct a `Choice(...)` node from two children. Size is derived from children.
    /// This represents an abstract interpretation choice between two possible values.
    pub fn choice(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        let left = left.into_rc();
        let right = right.into_rc();
        let s = std::cmp::max(left.size(), right.size());
        Value::Choice(Choice(left, right, s))
    }

    /// Construct a `Xor(...)` node from two children. Size is derived from children.
    pub fn xor(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        let left = left.into_rc();
        let right = right.into_rc();
        let s = std::cmp::max(left.size(), right.size());
        Value::Xor(XorExpr(left, right, s))
    }

    /// Construct an `Or(...)` node from two children (bitwise OR). Size is derived from children.
    pub fn or(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        let left = left.into_rc();
        let right = right.into_rc();
        let s = std::cmp::max(left.size(), right.size());
        Value::Or(OrExpr(left, right, s))
    }

    /// Construct an `And(...)` node from two children. Size is derived from children.
    pub fn and(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        let left = left.into_rc();
        let right = right.into_rc();
        let s = std::cmp::max(left.size(), right.size());
        Value::And(AndExpr(left, right, s))
    }

    /// Construct a `BoolNegate(...)` node from a child.
    pub fn bool_negate(child: impl IntoRcValue) -> Self {
        Value::BoolNegate(BoolNegateExpr(child.into_rc()))
    }

    /// Construct a `BoolAnd(...)` node from two children.
    pub fn bool_and(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        Value::BoolAnd(BoolAndExpr(left.into_rc(), right.into_rc()))
    }

    /// Construct a `BoolOr(...)` node from two children.
    pub fn bool_or(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        Value::BoolOr(BoolOrExpr(left.into_rc(), right.into_rc()))
    }

    /// Construct a `BoolXor(...)` node from two children.
    pub fn bool_xor(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        Value::BoolXor(BoolXorExpr(left.into_rc(), right.into_rc()))
    }

    /// Construct a `Load(...)` node from a child.
    pub fn load(child: impl IntoRcValue, size: usize, space: u8) -> Self {
        let child = child.into_rc();
        Value::Load(Load(child, size, space))
    }

    /// Construct an `IntEqual(...)` node from two children.
    pub fn int_equal(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        Value::IntEqual(IntEqual(left.into_rc(), right.into_rc()))
    }

    /// Construct an `IntLess(...)` node from two children.
    pub fn int_less(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        Value::IntLess(IntLess(left.into_rc(), right.into_rc()))
    }

    /// Construct an `IntSLess(...)` node from two children.
    pub fn int_sless(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        Value::IntSLess(IntSLess(left.into_rc(), right.into_rc()))
    }

    /// Construct a `PopCount(...)` node from a child.
    pub fn popcount(child: impl IntoRcValue) -> Self {
        Value::PopCount(PopCount(child.into_rc()))
    }

    /// Construct an `Int2Comp(...)` node from a child.
    pub fn int_2comp(child: impl IntoRcValue) -> Self {
        let child = child.into_rc();
        let s = child.size();
        Value::Int2Comp(Int2CompExpr(child, s))
    }

    /// Construct an `IntNotEqual(...)` node from two children.
    pub fn int_not_equal(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        Value::IntNotEqual(IntNotEqual(left.into_rc(), right.into_rc()))
    }

    /// Construct an `IntLessEqual(...)` node from two children.
    pub fn int_less_equal(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        Value::IntLessEqual(IntLessEqual(left.into_rc(), right.into_rc()))
    }

    /// Construct an `IntSLessEqual(...)` node from two children.
    pub fn int_sless_equal(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        Value::IntSLessEqual(IntSLessEqual(left.into_rc(), right.into_rc()))
    }

    /// Construct an `IntCarry(...)` node from two children.
    pub fn int_carry(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        Value::IntCarry(IntCarry(left.into_rc(), right.into_rc()))
    }

    /// Construct an `IntSCarry(...)` node from two children.
    pub fn int_scarry(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        Value::IntSCarry(IntSCarry(left.into_rc(), right.into_rc()))
    }

    /// Construct an `IntSBorrow(...)` node from two children.
    pub fn int_sborrow(left: impl IntoRcValue, right: impl IntoRcValue) -> Self {
        Value::IntSBorrow(IntSBorrow(left.into_rc(), right.into_rc()))
    }

    /// Construct a `ZeroExtend(...)` node that zero-extends `inner` to `output_size` bytes.
    pub fn zero_extend(inner: impl IntoRcValue, output_size: usize) -> Self {
        Value::ZeroExtend(ZeroExtend(inner.into_rc(), output_size))
    }

    /// Construct a `SignExtend(...)` node that sign-extends `inner` to `output_size` bytes.
    pub fn sign_extend(inner: impl IntoRcValue, output_size: usize) -> Self {
        Value::SignExtend(SignExtend(inner.into_rc(), output_size))
    }

    /// Construct an `Extract(...)` node that extracts `output_size` bytes from `inner`
    /// starting at `byte_offset`.
    pub fn extract(inner: impl IntoRcValue, byte_offset: usize, output_size: usize) -> Self {
        Value::Extract(Extract(inner.into_rc(), byte_offset, output_size))
    }

    /// Construct an `IntLeftShift(...)` node shifting `inner` left by `shift_amount` bits.
    /// `output_size` is the size in bytes of the result.
    pub fn int_left_shift(
        inner: impl IntoRcValue,
        shift_amount: impl IntoRcValue,
        output_size: usize,
    ) -> Self {
        Value::IntLeftShift(IntLeftShiftExpr(
            inner.into_rc(),
            shift_amount.into_rc(),
            output_size,
        ))
    }

    /// Construct a value representing `parent` with `sub`'s bytes inserted at `byte_offset`.
    ///
    /// Semantics: `(parent & keep_mask) | (zero_extend(sub, parent_size) << (byte_offset * 8))`
    ///
    /// `parent` must strictly cover `sub`: `parent.size() > sub.size()` and
    /// `byte_offset + sub.size() <= parent.size()`.
    pub fn insert_bytes(
        parent: impl IntoRcValue,
        sub: impl IntoRcValue,
        byte_offset: usize,
    ) -> Self {
        let parent_rc = parent.into_rc();
        let sub_rc = sub.into_rc();
        let parent_size = parent_rc.size();
        let sub_size = sub_rc.size();
        let sub_bits = sub_size * 8;

        // Safe: sub strictly smaller than parent, so sub_bits < parent_size * 8 ≤ 64.
        let clear_mask_bits: u64 = ((1u64 << sub_bits) - 1) << (byte_offset * 8);
        let keep_mask_bits: u64 = !clear_mask_bits;
        let mask_val = Value::const_(keep_mask_bits as i64, parent_size);

        let cleared_parent = Value::and(parent_rc, mask_val);
        let extended_sub = Value::zero_extend(sub_rc, parent_size);
        let shifted_sub = if byte_offset > 0 {
            Value::int_left_shift(
                extended_sub,
                Value::const_((byte_offset * 8) as i64, parent_size),
                parent_size,
            )
        } else {
            extended_sub
        };

        Value::or(cleared_parent, shifted_sub)
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

    fn normalize_commutative_rc(left: Rc<Value>, right: Rc<Value>) -> (Rc<Value>, Rc<Value>, bool) {
        if left.as_ref().as_const().is_some() && right.as_ref().as_const().is_none() {
            (right, left, true)
        } else {
            (left, right, false)
        }
    }

    fn normalize_choice_rc(left: Rc<Value>, right: Rc<Value>) -> (Rc<Value>, Rc<Value>, bool) {
        if matches!(left.as_ref(), Value::Choice(_)) && !matches!(right.as_ref(), Value::Choice(_))
        {
            (right, left, true)
        } else {
            (left, right, false)
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
            Value::BoolNegate(_) => 10,
            Value::BoolAnd(_) => 11,
            Value::BoolOr(_) => 12,
            Value::BoolXor(_) => 13,
            Value::IntLeftShift(_) => 14,
            Value::IntRightShift(_) => 15,
            Value::IntSignedRightShift(_) => 16,
            Value::Load(_) => 17,
            Value::ZeroExtend(_) => 18,
            Value::SignExtend(_) => 19,
            Value::Extract(_) => 20,
            Value::Top => 21,
            Value::IntSLess(_) => 22,
            Value::IntEqual(_) => 23,
            Value::IntLess(_) => 24,
            Value::PopCount(_) => 25,
            Value::Int2Comp(_) => 26,
            Value::IntNotEqual(_) => 27,
            Value::IntLessEqual(_) => 28,
            Value::IntSLessEqual(_) => 29,
            Value::IntCarry(_) => 30,
            Value::IntSCarry(_) => 31,
            Value::IntSBorrow(_) => 32,
            Value::Bind(_) => 33,
        }
    }

    fn as_boolean_const(&self) -> Option<bool> {
        self.as_const().and_then(|vn| {
            if vn.size() == 1 {
                Some(vn.offset() != 0)
            } else {
                None
            }
        })
    }

    fn is_boolean_valued(&self) -> bool {
        match self {
            Value::Const(Const(vn)) => vn.size() == 1,
            Value::BoolNegate(_)
            | Value::BoolAnd(_)
            | Value::BoolOr(_)
            | Value::BoolXor(_)
            | Value::IntSLess(_)
            | Value::IntEqual(_)
            | Value::IntLess(_)
            | Value::IntNotEqual(_)
            | Value::IntLessEqual(_)
            | Value::IntSLessEqual(_)
            | Value::IntCarry(_)
            | Value::IntSCarry(_)
            | Value::IntSBorrow(_) => true,
            _ => false,
        }
    }

    fn bool_const(value: bool) -> Self {
        Value::make_const(value as i64, 1)
    }
}

/// Constructors that accept already-boxed `Rc<Value>` operands, avoiding an extra
/// `Rc::new` allocation when the caller already holds a reference-counted value.
impl Value {
    pub fn mul_rc(left: Rc<Self>, right: Rc<Self>) -> Self {
        let s = std::cmp::max(left.size(), right.size());
        Value::Mul(MulExpr(left, right, s))
    }

    pub fn add_rc(left: Rc<Self>, right: Rc<Self>) -> Self {
        let s = std::cmp::max(left.size(), right.size());
        Value::Add(AddExpr(left, right, s))
    }

    pub fn sub_rc(left: Rc<Self>, right: Rc<Self>) -> Self {
        let s = std::cmp::max(left.size(), right.size());
        Value::Sub(SubExpr(left, right, s))
    }

    pub fn xor_rc(left: Rc<Self>, right: Rc<Self>) -> Self {
        let s = std::cmp::max(left.size(), right.size());
        Value::Xor(XorExpr(left, right, s))
    }

    pub fn or_rc(left: Rc<Self>, right: Rc<Self>) -> Self {
        let s = std::cmp::max(left.size(), right.size());
        Value::Or(OrExpr(left, right, s))
    }

    pub fn and_rc(left: Rc<Self>, right: Rc<Self>) -> Self {
        let s = std::cmp::max(left.size(), right.size());
        Value::And(AndExpr(left, right, s))
    }
}

impl Mul for Value {
    type Output = Value;

    fn mul(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        Value::Mul(MulExpr(Rc::new(self), Rc::new(rhs), s))
    }
}

impl Add for Value {
    type Output = Value;

    fn add(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        Value::Add(AddExpr(Rc::new(self), Rc::new(rhs), s))
    }
}

impl BitXor for Value {
    type Output = Value;

    fn bitxor(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        Value::Xor(XorExpr(Rc::new(self), Rc::new(rhs), s))
    }
}

impl BitAnd for Value {
    type Output = Value;

    fn bitand(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        Value::And(AndExpr(Rc::new(self), Rc::new(rhs), s))
    }
}

impl std::ops::BitOr for Value {
    type Output = Value;

    fn bitor(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        Value::Or(OrExpr(Rc::new(self), Rc::new(rhs), s))
    }
}

impl Sub for Value {
    type Output = Value;

    fn sub(self, rhs: Self) -> Self::Output {
        let s = std::cmp::max(self.size(), rhs.size());
        Value::Sub(SubExpr(Rc::new(self), Rc::new(rhs), s))
    }
}


impl Value {
    #[cfg(test)]
    pub fn simplify(&self) -> Value {
        let rc = Rc::new(self.clone());
        let simplified = Value::simplify_shared(&rc);
        drop(rc);
        Rc::try_unwrap(simplified).unwrap_or_else(|r| (*r).clone())
    }

    /// Rc-aware simplify: returns the original `rc` when no rewrite is needed,
    /// avoiding heap allocations for unchanged subtrees.
    pub(crate) fn simplify_shared(rc: &Rc<Self>) -> Rc<Self> {
        match rc.as_ref() {
            Value::Entry(_)
            | Value::Const(_)
            | Value::Top
            | Value::Bind(_)
            | Value::Offset(_) => Rc::clone(rc),
            Value::Add(expr) => AddExpr::simplify_rc(rc, expr),
            Value::Sub(expr) => SubExpr::simplify_rc(rc, expr),
            Value::Mul(expr) => MulExpr::simplify_rc(rc, expr),
            Value::Choice(expr) => Choice::simplify_rc(rc, expr),
            Value::Xor(expr) => XorExpr::simplify_rc(rc, expr),
            Value::And(expr) => AndExpr::simplify_rc(rc, expr),
            Value::Or(expr) => OrExpr::simplify_rc(rc, expr),
            Value::BoolNegate(expr) => BoolNegateExpr::simplify_rc(rc, expr),
            Value::BoolAnd(expr) => BoolAndExpr::simplify_rc(rc, expr),
            Value::BoolOr(expr) => BoolOrExpr::simplify_rc(rc, expr),
            Value::BoolXor(expr) => BoolXorExpr::simplify_rc(rc, expr),
            Value::IntLeftShift(expr) => IntLeftShiftExpr::simplify_rc(rc, expr),
            Value::IntRightShift(expr) => IntRightShiftExpr::simplify_rc(rc, expr),
            Value::IntSignedRightShift(expr) => IntSignedRightShiftExpr::simplify_rc(rc, expr),
            Value::Load(expr) => Load::simplify_rc(rc, expr),
            Value::ZeroExtend(expr) => ZeroExtend::simplify_rc(rc, expr),
            Value::SignExtend(expr) => SignExtend::simplify_rc(rc, expr),
            Value::Extract(expr) => Extract::simplify_rc(rc, expr),
            Value::IntSLess(expr) => IntSLess::simplify_rc(rc, expr),
            Value::IntEqual(expr) => IntEqual::simplify_rc(rc, expr),
            Value::IntLess(expr) => IntLess::simplify_rc(rc, expr),
            Value::PopCount(expr) => PopCount::simplify_rc(rc, expr),
            Value::Int2Comp(expr) => Int2CompExpr::simplify_rc(rc, expr),
            Value::IntNotEqual(expr) => IntNotEqual::simplify_rc(rc, expr),
            Value::IntLessEqual(expr) => IntLessEqual::simplify_rc(rc, expr),
            Value::IntSLessEqual(expr) => IntSLessEqual::simplify_rc(rc, expr),
            Value::IntCarry(expr) => IntCarry::simplify_rc(rc, expr),
            Value::IntSCarry(expr) => IntSCarry::simplify_rc(rc, expr),
            Value::IntSBorrow(expr) => IntSBorrow::simplify_rc(rc, expr),
        }
    }

}

impl Simplify for AddExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let AddExpr(a_intern, b_intern, _) = inner;

        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);

        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        if let (Some(a_vn), Some(b_vn)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            let a = a_vn.offset() as i64;
            let b = b_vn.offset() as i64;
            let res = a.wrapping_add(b);
            let size = Value::derive_size_from(new_a.as_ref())
                .max(Value::derive_size_from(new_b.as_ref()));
            return Rc::new(Value::make_const(res, size as u32));
        }

        let (left, right, swapped) = Value::normalize_commutative_rc(new_a, new_b);

        if let Some(0) = right.as_ref().as_const().map(|vn| vn.offset() as i64) {
            return left;
        }

        // ((expr + #a) + #b) -> (expr + #(a + b))
        if let Value::Add(AddExpr(lli, llr, _)) = left.as_ref() {
            if let Some(inner_c) = llr.as_ref().as_const() {
                if let Some(outer_c) = right.as_ref().as_const() {
                    let res = (inner_c.offset() as i64).wrapping_add(outer_c.offset() as i64);
                    let size = lli.as_ref().size().max(inner_c.size());
                    let new_c = Rc::new(Value::make_const(res, size as u32));
                    let rebuilt = Rc::new(Value::Add(AddExpr(Rc::clone(lli), new_c, size)));
                    return Value::simplify_shared(&rebuilt);
                }
            }
        }

        // ((expr - #a) + #b) -> (expr - #(a - b)) or (expr + #(b - a))
        if let Value::Sub(SubExpr(expr, a, _)) = left.as_ref() {
            if let Some(a_vn) = a.as_ref().as_const() {
                if let Some(b_vn) = right.as_ref().as_const() {
                    let a_const = a_vn.offset() as i64;
                    let b = b_vn.offset() as i64;
                    let res = a_const.wrapping_sub(b);
                    let size = expr.as_ref().size().max(left.as_ref().size());
                    if res < 0 {
                        let new_c = Rc::new(Value::make_const(res.wrapping_neg(), size as u32));
                        let rebuilt =
                            Rc::new(Value::Add(AddExpr(Rc::clone(expr), new_c, size)));
                        return Value::simplify_shared(&rebuilt);
                    } else {
                        let new_c = Rc::new(Value::make_const(res, size as u32));
                        let rebuilt =
                            Rc::new(Value::Sub(SubExpr(Rc::clone(expr), new_c, size)));
                        return Value::simplify_shared(&rebuilt);
                    }
                }
            }
        }

        if !swapped && Rc::ptr_eq(&left, a_intern) && Rc::ptr_eq(&right, b_intern) {
            return Rc::clone(outer);
        }
        let s = left.as_ref().size().max(right.as_ref().size());
        Rc::new(Value::Add(AddExpr(left, right, s)))
    }
}

impl Simplify for SubExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let SubExpr(a_intern, b_intern, _) = inner;

        let left = Value::simplify_shared(a_intern);
        let right = Value::simplify_shared(b_intern);

        if matches!(left.as_ref(), Value::Top) || matches!(right.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        if let (Some(lv), Some(rv)) = (left.as_ref().as_const(), right.as_ref().as_const()) {
            let res = (lv.offset() as i64).wrapping_sub(rv.offset() as i64);
            let size = Value::derive_size_from(left.as_ref())
                .max(Value::derive_size_from(right.as_ref()));
            return Rc::new(Value::make_const(res, size as u32));
        }

        // expr - 0 -> expr; expr - (-|a|) -> expr + a
        match right.as_ref().as_const().map(|vn| vn.offset() as i64) {
            Some(0) => return left,
            Some(a) if a < 0 => {
                let size = left.as_ref().size();
                let new_c = Rc::new(Value::make_const(a.wrapping_neg(), size as u32));
                let rebuilt = Rc::new(Value::Add(AddExpr(Rc::clone(&left), new_c, size)));
                return Value::simplify_shared(&rebuilt);
            }
            _ => {}
        }

        // x - x -> 0
        if left == right {
            let size = Value::derive_size_from(left.as_ref());
            return Rc::new(Value::make_const(0, size as u32));
        }

        // ((expr + #a) - #b) -> (expr + #(a - b)) or (expr - #(b - a))
        if let Value::Add(AddExpr(expr, a, _)) = left.as_ref() {
            if let Some(a_vn) = a.as_ref().as_const() {
                if let Some(b_vn) = right.as_ref().as_const() {
                    let res = (a_vn.offset() as i64).wrapping_sub(b_vn.offset() as i64);
                    let size = expr.as_ref().size().max(left.as_ref().size());
                    if res < 0 {
                        let new_c = Rc::new(Value::make_const(res.wrapping_neg(), size as u32));
                        let rebuilt =
                            Rc::new(Value::Sub(SubExpr(Rc::clone(expr), new_c, size)));
                        return Value::simplify_shared(&rebuilt);
                    } else {
                        let new_c = Rc::new(Value::make_const(res, size as u32));
                        let rebuilt =
                            Rc::new(Value::Add(AddExpr(Rc::clone(expr), new_c, size)));
                        return Value::simplify_shared(&rebuilt);
                    }
                }
            }
        }

        // ((expr - #a) - #b) -> (expr - #(a + b))
        if let Value::Sub(SubExpr(expr, a, _)) = left.as_ref() {
            if let Some(a_vn) = a.as_ref().as_const() {
                if let Some(b_vn) = right.as_ref().as_const() {
                    let res = (a_vn.offset() as i64).wrapping_add(b_vn.offset() as i64);
                    let size = expr.as_ref().size().max(left.as_ref().size());
                    let new_c = Rc::new(Value::make_const(res, size as u32));
                    let rebuilt = Rc::new(Value::Sub(SubExpr(Rc::clone(expr), new_c, size)));
                    return Value::simplify_shared(&rebuilt);
                }
            }
        }

        if Rc::ptr_eq(&left, a_intern) && Rc::ptr_eq(&right, b_intern) {
            return Rc::clone(outer);
        }
        let s = left.as_ref().size().max(right.as_ref().size());
        Rc::new(Value::Sub(SubExpr(left, right, s)))
    }
}

impl Simplify for MulExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let MulExpr(a_intern, b_intern, _) = inner;

        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);

        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        let (left, right, swapped) = Value::normalize_commutative_rc(new_a, new_b);

        if let (Some(av), Some(bv)) = (left.as_ref().as_const(), right.as_ref().as_const()) {
            let res = (av.offset() as i64).wrapping_mul(bv.offset() as i64);
            let size = Value::derive_size_from(left.as_ref())
                .max(Value::derive_size_from(right.as_ref()));
            return Rc::new(Value::make_const(res, size as u32));
        }

        if right.as_ref().as_const().map(|vn| vn.offset() as i64) == Some(1) {
            return left;
        }

        if right.as_ref().as_const().map(|vn| vn.offset() as i64) == Some(0) {
            let size = Value::derive_size_from(left.as_ref());
            return Rc::new(Value::make_const(0, size as u32));
        }

        if !swapped && Rc::ptr_eq(&left, a_intern) && Rc::ptr_eq(&right, b_intern) {
            return Rc::clone(outer);
        }
        let s = left.as_ref().size().max(right.as_ref().size());
        Rc::new(Value::Mul(MulExpr(left, right, s)))
    }
}

impl Simplify for Choice {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let Choice(a_intern, b_intern, _) = inner;

        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);

        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        let (mut left, mut right, mut swapped) = Value::normalize_choice_rc(new_a, new_b);

        if !matches!(left.as_ref(), Value::Choice(_))
            && !matches!(right.as_ref(), Value::Choice(_))
            && Value::variant_rank(left.as_ref()) > Value::variant_rank(right.as_ref())
        {
            std::mem::swap(&mut left, &mut right);
            swapped = !swapped;
        }

        if left == right {
            return left;
        }

        // Collapse nested duplicates: Choice(a, Choice(a, b)) -> Choice(a, b)
        if let Value::Choice(Choice(inner_a, inner_b, _)) = right.as_ref() {
            if inner_a.as_ref() == left.as_ref() {
                let rebuilt = Rc::new(Value::Choice(Choice(
                    Rc::clone(&left),
                    Rc::clone(inner_b),
                    right.as_ref().size(),
                )));
                return Value::simplify_shared(&rebuilt);
            }
            if inner_b.as_ref() == left.as_ref() {
                let rebuilt = Rc::new(Value::Choice(Choice(
                    Rc::clone(&left),
                    Rc::clone(inner_a),
                    right.as_ref().size(),
                )));
                return Value::simplify_shared(&rebuilt);
            }
        }

        // Factor common child between two Choices
        if let (Value::Choice(Choice(l1, l2, _)), Value::Choice(Choice(r1, r2, _))) =
            (left.as_ref(), right.as_ref())
        {
            let make_factored = |common: &Rc<Value>, x: &Rc<Value>, y: &Rc<Value>| {
                let inner_s = x.as_ref().size().max(y.as_ref().size());
                let inner = Rc::new(Value::Choice(Choice(Rc::clone(x), Rc::clone(y), inner_s)));
                let inner_simplified = Value::simplify_shared(&inner);
                let s = common.as_ref().size().max(inner_simplified.as_ref().size());
                let top = Rc::new(Value::Choice(Choice(
                    Rc::clone(common),
                    inner_simplified,
                    s,
                )));
                Value::simplify_shared(&top)
            };
            if l1.as_ref() == r1.as_ref() {
                return make_factored(l1, l2, r2);
            }
            if l1.as_ref() == r2.as_ref() {
                return make_factored(l1, l2, r1);
            }
            if l2.as_ref() == r1.as_ref() {
                return make_factored(l2, l1, r2);
            }
            if l2.as_ref() == r2.as_ref() {
                return make_factored(l2, l1, r1);
            }
        }

        if !swapped && Rc::ptr_eq(&left, a_intern) && Rc::ptr_eq(&right, b_intern) {
            return Rc::clone(outer);
        }
        let s = left.as_ref().size().max(right.as_ref().size());
        Rc::new(Value::Choice(Choice(left, right, s)))
    }
}

impl Simplify for XorExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let XorExpr(a_intern, b_intern, _) = inner;

        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);

        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        let (left, right, swapped) = Value::normalize_commutative_rc(new_a, new_b);

        if let (Some(lv), Some(rv)) = (left.as_ref().as_const(), right.as_ref().as_const()) {
            let res = (lv.offset() ^ rv.offset()) as i64;
            let size = Value::derive_size_from(left.as_ref())
                .max(Value::derive_size_from(right.as_ref()));
            return Rc::new(Value::make_const(res, size as u32));
        }

        if left == right {
            let size = Value::derive_size_from(left.as_ref());
            return Rc::new(Value::make_const(0, size as u32));
        }

        if right.as_ref().as_const().map(|vn| vn.offset()) == Some(0) {
            return left;
        }

        if !swapped && Rc::ptr_eq(&left, a_intern) && Rc::ptr_eq(&right, b_intern) {
            return Rc::clone(outer);
        }
        let s = left.as_ref().size().max(right.as_ref().size());
        Rc::new(Value::Xor(XorExpr(left, right, s)))
    }
}

impl Simplify for AndExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let AndExpr(a_intern, b_intern, _) = inner;

        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);

        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        let (left, right, swapped) = Value::normalize_commutative_rc(new_a, new_b);

        if let (Some(lv), Some(rv)) = (left.as_ref().as_const(), right.as_ref().as_const()) {
            let res = (lv.offset() & rv.offset()) as i64;
            let size = Value::derive_size_from(left.as_ref())
                .max(Value::derive_size_from(right.as_ref()));
            return Rc::new(Value::make_const(res, size as u32));
        }

        if left == right {
            return left;
        }

        if right.as_ref().as_const().map(|vn| vn.offset()) == Some(0) {
            let size = Value::derive_size_from(left.as_ref());
            return Rc::new(Value::make_const(0, size as u32));
        }

        let all_ones = match left.as_ref().size() {
            1 => Some(0xFF_u64),
            2 => Some(0xFFFF_u64),
            4 => Some(0xFFFF_FFFF_u64),
            8 => Some(u64::MAX),
            _ => None,
        };
        if let Some(mask) = all_ones {
            if right.as_ref().as_const().map(|vn| vn.offset()) == Some(mask) {
                return left;
            }
        }

        // Rule A: And(And(x, c1), c2) → And(x, c1 & c2)
        // Collapses the nested constant masks produced by successive insert_bytes calls.
        if let Some(rv) = right.as_ref().as_const() {
            if let Value::And(AndExpr(inner_x, inner_m, _)) = left.as_ref() {
                if let Some(inner_mv) = inner_m.as_ref().as_const() {
                    let folded = (rv.offset() & inner_mv.offset()) as i64;
                    let sz = left.as_ref().size();
                    let new_and = Rc::new(Value::And(AndExpr(
                        Rc::clone(inner_x),
                        Rc::new(Value::make_const(folded, sz as u32)),
                        sz,
                    )));
                    return Value::simplify_shared(&new_and);
                }
            }
        }

        // Rule B: And(x, c_mask) → x when the mask clears only bits already zero in x.
        // Handles And(ZeroExtend(sub, s), keep_mask) and And(Shift(ZExt(...), n), keep_mask)
        // produced during insert_bytes + distribute (Rule C).
        if let Some(mask_val) = right.as_ref().as_const() {
            let non_zero = !known_zero_bits(left.as_ref());
            if !mask_val.offset() & non_zero == 0 {
                return left;
            }
        }

        // Rule C: And(Or(x, y), c_mask) → Or(And(x, c_mask), And(y, c_mask))
        // Only distributes when at least one Or child has known structure (known_zero_bits ≠ 0)
        // so we don't expand And(Or(entry_a, entry_b), mask) unnecessarily.
        if let Value::Or(OrExpr(or_l, or_r, _)) = left.as_ref() {
            if right.as_ref().as_const().is_some()
                && (known_zero_bits(or_l.as_ref()) != 0
                    || known_zero_bits(or_r.as_ref()) != 0)
            {
                let sl = or_l.as_ref().size().max(right.as_ref().size());
                let and_l = Rc::new(Value::And(AndExpr(
                    Rc::clone(or_l),
                    Rc::clone(&right),
                    sl,
                )));
                let new_l = Value::simplify_shared(&and_l);
                let sr = or_r.as_ref().size().max(right.as_ref().size());
                let and_r = Rc::new(Value::And(AndExpr(
                    Rc::clone(or_r),
                    Rc::clone(&right),
                    sr,
                )));
                let new_r = Value::simplify_shared(&and_r);
                if new_l == new_r {
                    return new_l;
                }
                let s = new_l.as_ref().size().max(new_r.as_ref().size());
                // Children are already simplified; do not call simplify_shared on the Or
                // to avoid redundant traversal.
                return Rc::new(Value::Or(OrExpr(new_l, new_r, s)));
            }
        }

        if !swapped && Rc::ptr_eq(&left, a_intern) && Rc::ptr_eq(&right, b_intern) {
            return Rc::clone(outer);
        }
        let s = left.as_ref().size().max(right.as_ref().size());
        Rc::new(Value::And(AndExpr(left, right, s)))
    }
}

impl Simplify for OrExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let OrExpr(a_intern, b_intern, _) = inner;

        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);

        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        let (left, right, swapped) = Value::normalize_commutative_rc(new_a, new_b);

        if let (Some(lv), Some(rv)) = (left.as_ref().as_const(), right.as_ref().as_const()) {
            let res = (lv.offset() | rv.offset()) as i64;
            let size = Value::derive_size_from(left.as_ref())
                .max(Value::derive_size_from(right.as_ref()));
            return Rc::new(Value::make_const(res, size as u32));
        }

        if left == right {
            return left;
        }

        if right.as_ref().as_const().map(|vn| vn.offset()) == Some(0) {
            return left;
        }

        let all_ones = match left.as_ref().size() {
            1 => Some(0xFF_u64),
            2 => Some(0xFFFF_u64),
            4 => Some(0xFFFF_FFFF_u64),
            8 => Some(u64::MAX),
            _ => None,
        };
        if let Some(mask) = all_ones {
            if right.as_ref().as_const().map(|vn| vn.offset()) == Some(mask) {
                let size = Value::derive_size_from(left.as_ref());
                return Rc::new(Value::make_const(mask as i64, size as u32));
            }
        }

        if !swapped && Rc::ptr_eq(&left, a_intern) && Rc::ptr_eq(&right, b_intern) {
            return Rc::clone(outer);
        }
        let s = left.as_ref().size().max(right.as_ref().size());
        Rc::new(Value::Or(OrExpr(left, right, s)))
    }
}

impl Simplify for BoolNegateExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let BoolNegateExpr(child_intern) = inner;
        let new_child = Value::simplify_shared(child_intern);

        if matches!(new_child.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        if let Some(value) = new_child.as_ref().as_boolean_const() {
            return Rc::new(Value::bool_const(!value));
        }

        if let Value::BoolNegate(BoolNegateExpr(inner2)) = new_child.as_ref() {
            if inner2.as_ref().is_boolean_valued() {
                return Rc::clone(inner2);
            }
        }

        match new_child.as_ref() {
            Value::IntEqual(IntEqual(a, b)) => {
                return Rc::new(Value::IntNotEqual(IntNotEqual(Rc::clone(a), Rc::clone(b))));
            }
            Value::IntNotEqual(IntNotEqual(a, b)) => {
                return Rc::new(Value::IntEqual(IntEqual(Rc::clone(a), Rc::clone(b))));
            }
            Value::IntLess(IntLess(a, b)) => {
                return Rc::new(Value::IntLessEqual(IntLessEqual(Rc::clone(b), Rc::clone(a))));
            }
            Value::IntLessEqual(IntLessEqual(a, b)) => {
                return Rc::new(Value::IntLess(IntLess(Rc::clone(b), Rc::clone(a))));
            }
            Value::IntSLess(IntSLess(a, b)) => {
                return Rc::new(Value::IntSLessEqual(IntSLessEqual(Rc::clone(b), Rc::clone(a))));
            }
            Value::IntSLessEqual(IntSLessEqual(a, b)) => {
                return Rc::new(Value::IntSLess(IntSLess(Rc::clone(b), Rc::clone(a))));
            }
            _ => {}
        }

        if Rc::ptr_eq(&new_child, child_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::BoolNegate(BoolNegateExpr(new_child)))
    }
}

impl Simplify for BoolAndExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let BoolAndExpr(a_intern, b_intern) = inner;

        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);

        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        let (left, right, swapped) = Value::normalize_commutative_rc(new_a, new_b);

        if let (Some(lb), Some(rb)) =
            (left.as_ref().as_boolean_const(), right.as_ref().as_boolean_const())
        {
            return Rc::new(Value::bool_const(lb && rb));
        }

        if left == right && left.as_ref().is_boolean_valued() {
            return left;
        }

        if let Some(false) = right.as_ref().as_boolean_const() {
            return Rc::new(Value::bool_const(false));
        }

        if let Some(true) = right.as_ref().as_boolean_const() {
            if left.as_ref().is_boolean_valued() {
                return left;
            }
        }

        if !swapped && Rc::ptr_eq(&left, a_intern) && Rc::ptr_eq(&right, b_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::BoolAnd(BoolAndExpr(left, right)))
    }
}

impl Simplify for BoolOrExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let BoolOrExpr(a_intern, b_intern) = inner;

        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);

        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        let (left, right, swapped) = Value::normalize_commutative_rc(new_a, new_b);

        if let (Some(lb), Some(rb)) =
            (left.as_ref().as_boolean_const(), right.as_ref().as_boolean_const())
        {
            return Rc::new(Value::bool_const(lb || rb));
        }

        if left == right && left.as_ref().is_boolean_valued() {
            return left;
        }

        if let Some(false) = right.as_ref().as_boolean_const() {
            if left.as_ref().is_boolean_valued() {
                return left;
            }
        }

        if let Some(true) = right.as_ref().as_boolean_const() {
            return Rc::new(Value::bool_const(true));
        }

        if !swapped && Rc::ptr_eq(&left, a_intern) && Rc::ptr_eq(&right, b_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::BoolOr(BoolOrExpr(left, right)))
    }
}

impl Simplify for BoolXorExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let BoolXorExpr(a_intern, b_intern) = inner;

        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);

        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        let (left, right, swapped) = Value::normalize_commutative_rc(new_a, new_b);

        if let (Some(lb), Some(rb)) =
            (left.as_ref().as_boolean_const(), right.as_ref().as_boolean_const())
        {
            return Rc::new(Value::bool_const(lb ^ rb));
        }

        if left == right && left.as_ref().is_boolean_valued() {
            return Rc::new(Value::bool_const(false));
        }

        if let Some(false) = right.as_ref().as_boolean_const() {
            if left.as_ref().is_boolean_valued() {
                return left;
            }
        }

        if let Some(true) = right.as_ref().as_boolean_const() {
            if left.as_ref().is_boolean_valued() {
                let negated = Rc::new(Value::BoolNegate(BoolNegateExpr(Rc::clone(&left))));
                return Value::simplify_shared(&negated);
            }
        }

        if !swapped && Rc::ptr_eq(&left, a_intern) && Rc::ptr_eq(&right, b_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::BoolXor(BoolXorExpr(left, right)))
    }
}

impl Simplify for IntLeftShiftExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let IntLeftShiftExpr(a_intern, b_intern, _) = inner;

        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);

        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        if let (Some(lv), Some(rv)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            let shift_amt = rv.offset();
            let size_bits = (lv.size() * 8) as u64;
            if shift_amt >= size_bits {
                let size = Value::derive_size_from(new_a.as_ref());
                return Rc::new(Value::make_const(0, size as u32));
            }
            let result = lv.offset().wrapping_shl(shift_amt as u32);
            let masked = result & mask_for_size(lv.size());
            let size = Value::derive_size_from(new_a.as_ref());
            return Rc::new(Value::make_const(masked as i64, size as u32));
        }

        if new_b.as_ref().as_const().map(|vn| vn.offset()) == Some(0) {
            return new_a;
        }

        if Rc::ptr_eq(&new_a, a_intern) && Rc::ptr_eq(&new_b, b_intern) {
            return Rc::clone(outer);
        }
        let s = new_a.as_ref().size().max(new_b.as_ref().size());
        Rc::new(Value::IntLeftShift(IntLeftShiftExpr(new_a, new_b, s)))
    }
}

impl Simplify for IntRightShiftExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let IntRightShiftExpr(a_intern, b_intern, _) = inner;

        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);

        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        if let (Some(lv), Some(rv)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            let shift_amt = rv.offset();
            let size_bits = (lv.size() * 8) as u64;
            if shift_amt >= size_bits {
                let size = Value::derive_size_from(new_a.as_ref());
                return Rc::new(Value::make_const(0, size as u32));
            }
            let result = lv.offset().wrapping_shr(shift_amt as u32);
            let size = Value::derive_size_from(new_a.as_ref());
            return Rc::new(Value::make_const(result as i64, size as u32));
        }

        if new_b.as_ref().as_const().map(|vn| vn.offset()) == Some(0) {
            return new_a;
        }

        if Rc::ptr_eq(&new_a, a_intern) && Rc::ptr_eq(&new_b, b_intern) {
            return Rc::clone(outer);
        }
        let s = new_a.as_ref().size().max(new_b.as_ref().size());
        Rc::new(Value::IntRightShift(IntRightShiftExpr(new_a, new_b, s)))
    }
}

impl Simplify for IntSignedRightShiftExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let IntSignedRightShiftExpr(a_intern, b_intern, _) = inner;

        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);

        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        if let (Some(lv), Some(rv)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            let left_val = lv.offset();
            let shift_amt = rv.offset();
            let size_bits = (lv.size() * 8) as u32;
            let signed_val = if size_bits < 64 {
                let sign_bit = 1u64 << (size_bits - 1);
                if left_val & sign_bit != 0 {
                    (left_val | (u64::MAX << size_bits)) as i64
                } else {
                    left_val as i64
                }
            } else {
                left_val as i64
            };
            if shift_amt >= size_bits as u64 {
                let result = if signed_val < 0 { -1i64 } else { 0i64 };
                let size = Value::derive_size_from(new_a.as_ref());
                return Rc::new(Value::make_const(result, size as u32));
            }
            let result = signed_val.wrapping_shr(shift_amt as u32);
            let masked = (result as u64) & mask_for_size(lv.size());
            let size = Value::derive_size_from(new_a.as_ref());
            return Rc::new(Value::make_const(masked as i64, size as u32));
        }

        if new_b.as_ref().as_const().map(|vn| vn.offset()) == Some(0) {
            return new_a;
        }

        if Rc::ptr_eq(&new_a, a_intern) && Rc::ptr_eq(&new_b, b_intern) {
            return Rc::clone(outer);
        }
        let s = new_a.as_ref().size().max(new_b.as_ref().size());
        Rc::new(Value::IntSignedRightShift(IntSignedRightShiftExpr(new_a, new_b, s)))
    }
}

impl Simplify for Load {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let Load(child_intern, size, space) = inner;
        let new_child = Value::simplify_shared(child_intern);
        if Rc::ptr_eq(&new_child, child_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::Load(Load(new_child, *size, *space)))
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

/// Return a bitmask where a 1-bit means "this bit is definitely zero in `val`".
///
/// Conservative: returns 0 for any expression whose zero-bits cannot be statically
/// determined. Used by the And/Extract simplifiers to detect redundant masking and
/// route bit-range extractions through Or nodes.
fn known_zero_bits(val: &Value) -> u64 {
    match val {
        Value::Const(c) => !c.offset(),
        Value::ZeroExtend(ZeroExtend(inner, _)) => !mask_for_size(inner.as_ref().size()),
        Value::IntLeftShift(IntLeftShiftExpr(inner, shift, _)) => {
            if let Some(c) = shift.as_ref().as_const() {
                let s = c.offset().min(63) as u32;
                let lower = if s == 0 { 0u64 } else { (1u64 << s) - 1 };
                let inner_z = known_zero_bits(inner.as_ref());
                lower | inner_z.checked_shl(s).unwrap_or(u64::MAX)
            } else {
                0
            }
        }
        Value::And(AndExpr(_, right, _)) => {
            if let Some(c) = right.as_ref().as_const() { !c.offset() } else { 0 }
        }
        Value::Or(OrExpr(l, r, _)) => {
            known_zero_bits(l.as_ref()) & known_zero_bits(r.as_ref())
        }
        _ => 0,
    }
}

impl Simplify for ZeroExtend {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let ZeroExtend(child_intern, output_size) = inner;
        let new_child = Value::simplify_shared(child_intern);

        if matches!(new_child.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        if new_child.as_ref().size() == *output_size {
            return new_child;
        }

        if let Some(vn) = new_child.as_ref().as_const() {
            let src_value = vn.offset() & mask_for_size(vn.size());
            return Rc::new(Value::make_const(src_value as i64, *output_size as u32));
        }

        if let Value::ZeroExtend(ZeroExtend(inner2, s1)) = new_child.as_ref() {
            if *output_size >= *s1 {
                let rebuilt =
                    Rc::new(Value::ZeroExtend(ZeroExtend(Rc::clone(inner2), *output_size)));
                return Value::simplify_shared(&rebuilt);
            }
        }

        if Rc::ptr_eq(&new_child, child_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::ZeroExtend(ZeroExtend(new_child, *output_size)))
    }
}

impl Simplify for SignExtend {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let SignExtend(child_intern, output_size) = inner;
        let new_child = Value::simplify_shared(child_intern);

        if matches!(new_child.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        if new_child.as_ref().size() == *output_size {
            return new_child;
        }

        if let Some(vn) = new_child.as_ref().as_const() {
            let src_size = vn.size();
            let raw = vn.offset();
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
            return Rc::new(Value::make_const(masked as i64, *output_size as u32));
        }

        if let Value::SignExtend(SignExtend(inner2, s1)) = new_child.as_ref() {
            if *output_size >= *s1 {
                let rebuilt =
                    Rc::new(Value::SignExtend(SignExtend(Rc::clone(inner2), *output_size)));
                return Value::simplify_shared(&rebuilt);
            }
        }

        if Rc::ptr_eq(&new_child, child_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::SignExtend(SignExtend(new_child, *output_size)))
    }
}

impl Simplify for Extract {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let Extract(child_intern, byte_offset, output_size) = inner;
        let new_child = Value::simplify_shared(child_intern);

        if matches!(new_child.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }

        if *byte_offset == 0 && new_child.as_ref().size() == *output_size {
            return new_child;
        }

        if *byte_offset == 0 {
            if let Some(e) = new_child.as_ref().as_entry() {
                let vn = *e.deref();
                let vn = VarNode::new(vn.offset(), *output_size, vn.space_index());
                return Rc::new(Value::entry(vn));
            }

            if let Some(e) = new_child.as_ref().as_offset() {
                let new_size = VarNode::new_const(e.1.0.offset(), *output_size);
                return Rc::new(Value::offset(e.0.0, new_size));
            }

            if let Some(ZeroExtend(val, size)) = new_child.as_ref().as_zext() {
                if *output_size == val.as_ref().size() {
                    return Rc::clone(val);
                } else if *output_size < val.as_ref().size() {
                    let rebuilt =
                        Rc::new(Value::Extract(Extract(Rc::clone(val), 0, *output_size)));
                    return Value::simplify_shared(&rebuilt);
                } else if output_size <= size {
                    let rebuilt =
                        Rc::new(Value::ZeroExtend(ZeroExtend(Rc::clone(val), *output_size)));
                    return Value::simplify_shared(&rebuilt);
                }
            }

            if let Some(AddExpr(left, right, _)) = new_child.as_ref().as_add() {
                let left_ex =
                    Rc::new(Value::Extract(Extract(Rc::clone(left), 0, *output_size)));
                let left_s = Value::simplify_shared(&left_ex);
                let right_ex =
                    Rc::new(Value::Extract(Extract(Rc::clone(right), 0, *output_size)));
                let right_s = Value::simplify_shared(&right_ex);
                let size = left_s.as_ref().size().max(right_s.as_ref().size());
                let sum = Rc::new(Value::Add(AddExpr(left_s, right_s, size)));
                return Value::simplify_shared(&sum);
            }

            if let Some(SignExtend(val, size)) = new_child.as_ref().as_sext() {
                if *output_size == val.as_ref().size() {
                    return Rc::clone(val);
                } else if *output_size < val.as_ref().size() {
                    let rebuilt =
                        Rc::new(Value::Extract(Extract(Rc::clone(val), 0, *output_size)));
                    return Value::simplify_shared(&rebuilt);
                } else if output_size <= size {
                    let rebuilt =
                        Rc::new(Value::SignExtend(SignExtend(Rc::clone(val), *output_size)));
                    return Value::simplify_shared(&rebuilt);
                }
            }
        }

        // Rule E: extract(Shift(x, shift_const), off, size) when shift is byte-aligned.
        // Routes extraction through a left-shift, recovering the pre-shift value for reads
        // that fall exactly within the shifted region — e.g. reading AH after an insert_bytes.
        if let Value::IntLeftShift(IntLeftShiftExpr(shift_inner, shift_n, _)) =
            new_child.as_ref()
        {
            if let Some(shift_bits) = shift_n.as_ref().as_const().map(|c| c.offset()) {
                if shift_bits % 8 == 0 {
                    let shift_bytes = (shift_bits / 8) as usize;
                    let inner_size = shift_inner.as_ref().size();
                    if *byte_offset >= shift_bytes
                        && byte_offset + output_size <= shift_bytes + inner_size
                    {
                        let new_off = byte_offset - shift_bytes;
                        let ex = Rc::new(Value::Extract(Extract(
                            Rc::clone(shift_inner),
                            new_off,
                            *output_size,
                        )));
                        return Value::simplify_shared(&ex);
                    }
                    if byte_offset + output_size <= shift_bytes {
                        return Rc::new(Value::make_const(0, *output_size as u32));
                    }
                }
            }
        }

        // Rule D: extract(Or(x, y), off, size) → extract from the side whose bits are
        // non-zero in the extraction range. Uses known_zero_bits to detect that the other
        // side contributes nothing — e.g. reading AL from Or(And(RAX, keep_mask), ZExt(al,8)).
        if let Value::Or(OrExpr(or_l, or_r, _)) = new_child.as_ref() {
            let extraction_mask = mask_for_size(*output_size)
                .checked_shl((byte_offset * 8) as u32)
                .unwrap_or(0);
            if known_zero_bits(or_l.as_ref()) & extraction_mask == extraction_mask {
                let ex = Rc::new(Value::Extract(Extract(
                    Rc::clone(or_r),
                    *byte_offset,
                    *output_size,
                )));
                return Value::simplify_shared(&ex);
            }
            if known_zero_bits(or_r.as_ref()) & extraction_mask == extraction_mask {
                let ex = Rc::new(Value::Extract(Extract(
                    Rc::clone(or_l),
                    *byte_offset,
                    *output_size,
                )));
                return Value::simplify_shared(&ex);
            }
        }

        if let Some(vn) = new_child.as_ref().as_const() {
            let shift_amt = byte_offset.saturating_mul(8) as u32;
            let shifted = vn.offset().checked_shr(shift_amt).unwrap_or(0);
            let masked = shifted & mask_for_size(*output_size);
            return Rc::new(Value::make_const(masked as i64, *output_size as u32));
        }

        if Rc::ptr_eq(&new_child, child_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::Extract(Extract(new_child, *byte_offset, *output_size)))
    }
}

impl Simplify for IntEqual {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let IntEqual(a_intern, b_intern) = inner;
        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);
        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }
        if let (Some(av), Some(bv)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            return Rc::new(Value::make_const((av.offset() == bv.offset()) as i64, 1));
        }
        if new_a == new_b {
            return Rc::new(Value::make_const(1, 1));
        }
        if Rc::ptr_eq(&new_a, a_intern) && Rc::ptr_eq(&new_b, b_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::IntEqual(IntEqual(new_a, new_b)))
    }
}

impl Simplify for IntLess {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let IntLess(a_intern, b_intern) = inner;
        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);
        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }
        if let (Some(av), Some(bv)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            return Rc::new(Value::make_const((av.offset() < bv.offset()) as i64, 1));
        }
        if new_a == new_b {
            return Rc::new(Value::make_const(0, 1));
        }
        if Rc::ptr_eq(&new_a, a_intern) && Rc::ptr_eq(&new_b, b_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::IntLess(IntLess(new_a, new_b)))
    }
}

impl Simplify for IntSLess {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let IntSLess(a_intern, b_intern) = inner;
        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);
        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }
        if let (Some(av), Some(bv)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            return Rc::new(Value::make_const(
                ((av.offset() as i64) < (bv.offset() as i64)) as i64,
                1,
            ));
        }
        if new_a == new_b {
            return Rc::new(Value::make_const(0, 1));
        }
        if Rc::ptr_eq(&new_a, a_intern) && Rc::ptr_eq(&new_b, b_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::IntSLess(IntSLess(new_a, new_b)))
    }
}

impl Simplify for PopCount {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let PopCount(child_intern) = inner;
        let new_child = Value::simplify_shared(child_intern);
        if matches!(new_child.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }
        if let Some(vn) = new_child.as_ref().as_const() {
            return Rc::new(Value::make_const(vn.offset().count_ones() as i64, 1));
        }
        if Rc::ptr_eq(&new_child, child_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::PopCount(PopCount(new_child)))
    }
}

impl Simplify for IntNotEqual {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let IntNotEqual(a_intern, b_intern) = inner;
        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);
        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }
        if let (Some(av), Some(bv)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            return Rc::new(Value::make_const((av.offset() != bv.offset()) as i64, 1));
        }
        if new_a == new_b {
            return Rc::new(Value::make_const(0, 1));
        }
        if Rc::ptr_eq(&new_a, a_intern) && Rc::ptr_eq(&new_b, b_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::IntNotEqual(IntNotEqual(new_a, new_b)))
    }
}

impl Simplify for IntLessEqual {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let IntLessEqual(a_intern, b_intern) = inner;
        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);
        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }
        if let (Some(av), Some(bv)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            return Rc::new(Value::make_const((av.offset() <= bv.offset()) as i64, 1));
        }
        if new_a == new_b {
            return Rc::new(Value::make_const(1, 1));
        }
        if Rc::ptr_eq(&new_a, a_intern) && Rc::ptr_eq(&new_b, b_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::IntLessEqual(IntLessEqual(new_a, new_b)))
    }
}

impl Simplify for IntSLessEqual {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let IntSLessEqual(a_intern, b_intern) = inner;
        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);
        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }
        if let (Some(av), Some(bv)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            return Rc::new(Value::make_const(
                ((av.offset() as i64) <= (bv.offset() as i64)) as i64,
                1,
            ));
        }
        if new_a == new_b {
            return Rc::new(Value::make_const(1, 1));
        }
        if Rc::ptr_eq(&new_a, a_intern) && Rc::ptr_eq(&new_b, b_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::IntSLessEqual(IntSLessEqual(new_a, new_b)))
    }
}

impl Simplify for IntCarry {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let IntCarry(a_intern, b_intern) = inner;
        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);
        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }
        if let (Some(av), Some(bv)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            let bits = (av.size() * 8) as u32;
            let carry = (av.offset() as u128 + bv.offset() as u128) >> bits;
            return Rc::new(Value::make_const((carry != 0) as i64, 1));
        }
        if Rc::ptr_eq(&new_a, a_intern) && Rc::ptr_eq(&new_b, b_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::IntCarry(IntCarry(new_a, new_b)))
    }
}

impl Simplify for IntSCarry {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let IntSCarry(a_intern, b_intern) = inner;
        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);
        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }
        if let (Some(av), Some(bv)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            let n = av.size() * 8;
            let mask = if n == 64 {
                u64::MAX
            } else {
                (1u64 << n).wrapping_sub(1)
            };
            let sign_mask = 1u64 << (n - 1);
            let a_val = av.offset() & mask;
            let b_val = bv.offset() & mask;
            let sum = a_val.wrapping_add(b_val) & mask;
            let overflow = ((a_val ^ sum) & (b_val ^ sum) & sign_mask) != 0;
            return Rc::new(Value::make_const(overflow as i64, 1));
        }
        if Rc::ptr_eq(&new_a, a_intern) && Rc::ptr_eq(&new_b, b_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::IntSCarry(IntSCarry(new_a, new_b)))
    }
}

impl Simplify for IntSBorrow {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let IntSBorrow(a_intern, b_intern) = inner;
        let new_a = Value::simplify_shared(a_intern);
        let new_b = Value::simplify_shared(b_intern);
        if matches!(new_a.as_ref(), Value::Top) || matches!(new_b.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }
        if let (Some(av), Some(bv)) = (new_a.as_ref().as_const(), new_b.as_ref().as_const()) {
            let n = av.size() * 8;
            let mask = if n == 64 {
                u64::MAX
            } else {
                (1u64 << n).wrapping_sub(1)
            };
            let sign_mask = 1u64 << (n - 1);
            let a_val = av.offset() & mask;
            let b_val = bv.offset() & mask;
            let diff = a_val.wrapping_sub(b_val) & mask;
            let overflow = ((a_val ^ b_val) & (a_val ^ diff) & sign_mask) != 0;
            return Rc::new(Value::make_const(overflow as i64, 1));
        }
        if new_a == new_b {
            return Rc::new(Value::make_const(0, 1));
        }
        if Rc::ptr_eq(&new_a, a_intern) && Rc::ptr_eq(&new_b, b_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::IntSBorrow(IntSBorrow(new_a, new_b)))
    }
}

impl Simplify for Int2CompExpr {
    fn simplify_rc(outer: &Rc<Value>, inner: &Self) -> Rc<Value> {
        let Int2CompExpr(child_intern, output_size) = inner;
        let new_child = Value::simplify_shared(child_intern);
        if matches!(new_child.as_ref(), Value::Top) {
            return Rc::new(Value::Top);
        }
        if let Some(vn) = new_child.as_ref().as_const() {
            let negated = (vn.offset() as i64).wrapping_neg();
            return Rc::new(Value::make_const(negated, *output_size as u32));
        }
        // identity: int_2comp(int_2comp(x)) = x
        if let Value::Int2Comp(Int2CompExpr(inner2, _)) = new_child.as_ref() {
            return Rc::clone(inner2);
        }
        if Rc::ptr_eq(&new_child, child_intern) {
            return Rc::clone(outer);
        }
        Rc::new(Value::Int2Comp(Int2CompExpr(new_child, *output_size)))
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
                write!(f, "offset({},{})", vn.display(info), con.display(info))
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
            Value::BoolNegate(BoolNegateExpr(a)) => {
                write!(f, "!")?;
                fmt_operand_jingle(f, a.as_ref(), info)
            }
            Value::BoolAnd(BoolAndExpr(a, b)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "&&")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::BoolOr(BoolOrExpr(a, b)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "||")?;
                fmt_operand_jingle(f, b.as_ref(), info)
            }
            Value::BoolXor(BoolXorExpr(a, b)) => {
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, "^^")?;
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
            Value::Load(Load(a, size, space)) => {
                let name = info
                    .get_space(*space as usize)
                    .ok_or(std::fmt::Error)?
                    .name
                    .as_str();
                write!(f, "*[{name}]")?;
                fmt_operand_jingle(f, a.as_ref(), info)?;
                write!(f, ":{size}")
            }
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
            Value::Bind(u) => write!(f, "bind({}, {})", u.id(), u.size()),
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
            Value::BoolNegate(BoolNegateExpr(a)) => {
                write!(f, "!")?;
                fmt_operand(f, a.as_ref())
            }
            Value::BoolAnd(BoolAndExpr(a, b)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "&&")?;
                fmt_operand(f, b.as_ref())
            }
            Value::BoolOr(BoolOrExpr(a, b)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "||")?;
                fmt_operand(f, b.as_ref())
            }
            Value::BoolXor(BoolXorExpr(a, b)) => {
                fmt_operand(f, a.as_ref())?;
                write!(f, "^^")?;
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
            Value::Load(Load(a, size, space)) => {
                // Load(child)
                write!(f, "*[{space}]")?;
                fmt_operand(f, a.as_ref())?;
                write!(f, ":{size}")
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
            Value::Bind(u) => write!(f, "bind({}, {})", u.id(), u.size()),
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
            Value::BoolNegate(BoolNegateExpr(a)) => {
                write!(f, "!")?;
                fmt_operand_hex(f, a.as_ref())
            }
            Value::BoolAnd(BoolAndExpr(a, b)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "&&")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::BoolOr(BoolOrExpr(a, b)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "||")?;
                fmt_operand_hex(f, b.as_ref())
            }
            Value::BoolXor(BoolXorExpr(a, b)) => {
                fmt_operand_hex(f, a.as_ref())?;
                write!(f, "^^")?;
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
            Value::Load(Load(a, size, space)) => {
                write!(f, "*[{space:x}]{:x}:{size:x}", a.as_ref())
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
            Value::Bind(u) => write!(f, "bind({:x}, {})", u.id(), u.size()),
            Value::Top => write!(f, "⊤"),
        }
    }
}

impl JoinSemiLattice for Value {
    fn join(&mut self, _other: &Self) {}
}

#[cfg(test)]
#[path = "value_tests.rs"]
mod value_tests;
