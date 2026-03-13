use super::*;
use internment::Intern;
use jingle_sleigh::VarNode;

fn vn_a() -> VarNode {
    VarNode::new(0x100u64, 8u32, 0u32)
}

fn vn_b() -> VarNode {
    VarNode::new(0x200u64, 8u32, 0u32)
}

fn vn_c() -> VarNode {
    VarNode::new(0x300u64, 8u32, 0u32)
}

// --- AddExpr -----------------------------------------------------------------

#[test]
fn add_const_folding() {
    let result = (SimpleValue::const_(3) + SimpleValue::const_(4)).simplify();
    assert_eq!(result, SimpleValue::make_const(7, 8));
}

#[test]
fn add_identity_zero() {
    let result = (SimpleValue::entry(vn_a()) + SimpleValue::const_(0)).simplify();
    assert_eq!(result, SimpleValue::entry(vn_a()));
}

#[test]
fn add_top_left() {
    let result = (SimpleValue::Top + SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::Top);
}

#[test]
fn add_top_right() {
    let result = (SimpleValue::const_(1) + SimpleValue::Top).simplify();
    assert_eq!(result, SimpleValue::Top);
}

#[test]
fn add_normalizes_const_to_right() {
    let result = (SimpleValue::const_(5) + SimpleValue::entry(vn_a())).simplify();
    let add = result.as_add().expect("expected Add node");
    assert!(add.0.as_entry().is_some(), "expected entry on left");
    assert!(add.1.as_const().is_some(), "expected const on right");
}

#[test]
fn add_nested_const_folding() {
    let inner = SimpleValue::entry(vn_a()) + SimpleValue::const_(10);
    let result = (inner + SimpleValue::const_(5)).simplify();
    let add = result.as_add().expect("expected Add node");
    assert_eq!(add.0.as_ref(), &SimpleValue::entry(vn_a()));
    assert_eq!(add.1.as_ref().as_const_value(), Some(15));
}

#[test]
fn add_sub_interaction_positive() {
    let inner = SimpleValue::entry(vn_a()) - SimpleValue::const_(10);
    let result = (inner + SimpleValue::const_(3)).simplify();
    let sub = result.as_sub().expect("expected Sub node");
    assert_eq!(sub.0.as_ref(), &SimpleValue::entry(vn_a()));
    assert_eq!(sub.1.as_ref().as_const_value(), Some(7));
}

#[test]
fn add_sub_interaction_negative() {
    let inner = SimpleValue::entry(vn_a()) - SimpleValue::const_(3);
    let result = (inner + SimpleValue::const_(10)).simplify();
    let add = result.as_add().expect("expected Add node");
    assert_eq!(add.0.as_ref(), &SimpleValue::entry(vn_a()));
    assert_eq!(add.1.as_ref().as_const_value(), Some(7));
}

#[test]
fn add_wrapping_overflow() {
    let result = (SimpleValue::const_(i64::MAX) + SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::make_const(i64::MIN, 8));
}

#[test]
fn add_negative_const() {
    let result = (SimpleValue::const_(-5) + SimpleValue::const_(3)).simplify();
    assert_eq!(result, SimpleValue::make_const(-2, 8));
}

#[test]
fn add_double_top() {
    let result = (SimpleValue::Top + SimpleValue::Top).simplify();
    assert_eq!(result, SimpleValue::Top);
}

// --- SubExpr -----------------------------------------------------------------

#[test]
fn sub_const_folding() {
    let result = (SimpleValue::const_(10) - SimpleValue::const_(3)).simplify();
    assert_eq!(result, SimpleValue::make_const(7, 8));
}

#[test]
fn sub_identity_zero() {
    let result = (SimpleValue::entry(vn_a()) - SimpleValue::const_(0)).simplify();
    assert_eq!(result, SimpleValue::entry(vn_a()));
}

#[test]
fn sub_negative_const() {
    // entry - (-5)  =>  entry + 5
    let result = (SimpleValue::entry(vn_a()) - SimpleValue::const_(-5)).simplify();
    let add = result.as_add().expect("expected Add node");
    assert_eq!(add.0.as_ref(), &SimpleValue::entry(vn_a()));
    assert_eq!(add.1.as_ref().as_const_value(), Some(5));
}

#[test]
fn sub_self() {
    let result = (SimpleValue::entry(vn_a()) - SimpleValue::entry(vn_a())).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 8));
}

#[test]
fn sub_top_left() {
    let result = (SimpleValue::Top - SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::Top);
}

#[test]
fn sub_top_right() {
    let result = (SimpleValue::const_(1) - SimpleValue::Top).simplify();
    assert_eq!(result, SimpleValue::Top);
}

#[test]
fn sub_nested_add_const_positive() {
    // (entry + 10) - 3  =>  entry + 7
    let inner = SimpleValue::entry(vn_a()) + SimpleValue::const_(10);
    let result = (inner - SimpleValue::const_(3)).simplify();
    let add = result.as_add().expect("expected Add node");
    assert_eq!(add.0.as_ref(), &SimpleValue::entry(vn_a()));
    assert_eq!(add.1.as_ref().as_const_value(), Some(7));
}

#[test]
fn sub_nested_add_const_negative() {
    // (entry + 3) - 10  =>  entry - 7
    let inner = SimpleValue::entry(vn_a()) + SimpleValue::const_(3);
    let result = (inner - SimpleValue::const_(10)).simplify();
    let sub = result.as_sub().expect("expected Sub node");
    assert_eq!(sub.0.as_ref(), &SimpleValue::entry(vn_a()));
    assert_eq!(sub.1.as_ref().as_const_value(), Some(7));
}

#[test]
fn sub_nested_sub_const() {
    // (entry - 3) - 4  =>  entry - 7
    let inner = SimpleValue::entry(vn_a()) - SimpleValue::const_(3);
    let result = (inner - SimpleValue::const_(4)).simplify();
    let sub = result.as_sub().expect("expected Sub node");
    assert_eq!(sub.0.as_ref(), &SimpleValue::entry(vn_a()));
    assert_eq!(sub.1.as_ref().as_const_value(), Some(7));
}

#[test]
fn sub_wrapping_underflow() {
    let result = (SimpleValue::const_(i64::MIN) - SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::make_const(i64::MAX, 8));
}

// --- MulExpr -----------------------------------------------------------------

#[test]
fn mul_const_folding() {
    let result = (SimpleValue::const_(6) * SimpleValue::const_(7)).simplify();
    assert_eq!(result, SimpleValue::make_const(42, 8));
}

#[test]
fn mul_identity_one() {
    let result = (SimpleValue::entry(vn_a()) * SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::entry(vn_a()));
}

#[test]
fn mul_zero() {
    let result = (SimpleValue::entry(vn_a()) * SimpleValue::const_(0)).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 8));
}

#[test]
fn mul_top_left() {
    let result = (SimpleValue::Top * SimpleValue::const_(5)).simplify();
    assert_eq!(result, SimpleValue::Top);
}

#[test]
fn mul_top_right() {
    let result = (SimpleValue::const_(5) * SimpleValue::Top).simplify();
    assert_eq!(result, SimpleValue::Top);
}

#[test]
fn mul_normalizes_const_to_right() {
    let result = (SimpleValue::const_(3) * SimpleValue::entry(vn_a())).simplify();
    let mul = result.as_mul().expect("expected Mul node");
    assert!(mul.0.as_entry().is_some(), "expected entry on left");
    assert!(mul.1.as_const().is_some(), "expected const on right");
}

#[test]
fn mul_negative() {
    let result = (SimpleValue::const_(-3) * SimpleValue::const_(4)).simplify();
    assert_eq!(result, SimpleValue::make_const(-12, 8));
}

#[test]
fn mul_wrapping() {
    let result = (SimpleValue::const_(i64::MAX) * SimpleValue::const_(2)).simplify();
    assert_eq!(result, SimpleValue::make_const(i64::MAX.wrapping_mul(2), 8));
}

// --- Or ----------------------------------------------------------------------

#[test]
fn or_identical_children() {
    let a = SimpleValue::entry(vn_a());
    let result = SimpleValue::or(a.clone(), a).simplify();
    assert_eq!(result, SimpleValue::entry(vn_a()));
}

#[test]
fn or_top_left() {
    let a = SimpleValue::entry(vn_a());
    let result = SimpleValue::or(SimpleValue::Top, a).simplify();
    assert_eq!(result, SimpleValue::Top);
}

#[test]
fn or_top_right() {
    let a = SimpleValue::entry(vn_a());
    let result = SimpleValue::or(a, SimpleValue::Top).simplify();
    assert_eq!(result, SimpleValue::Top);
}

#[test]
fn or_nested_duplicate_inner_left() {
    // Or(a, Or(a, b))  =>  Or(a, b)
    let a = SimpleValue::entry(vn_a());
    let b = SimpleValue::entry(vn_b());
    let inner = SimpleValue::or(a.clone(), b.clone());
    let result = SimpleValue::or(a.clone(), inner).simplify();
    let or = result.as_or().expect("expected Or");
    assert_eq!(or.0.as_ref(), &a);
    assert_eq!(or.1.as_ref(), &b);
}

#[test]
fn or_nested_duplicate_inner_right() {
    // Or(a, Or(b, a))  =>  Or(a, b)
    let a = SimpleValue::entry(vn_a());
    let b = SimpleValue::entry(vn_b());
    let inner = SimpleValue::or(b.clone(), a.clone());
    let result = SimpleValue::or(a.clone(), inner).simplify();
    let or = result.as_or().expect("expected Or");
    assert_eq!(or.0.as_ref(), &a);
    assert_eq!(or.1.as_ref(), &b);
}

#[test]
fn or_common_factor_l1_r1() {
    // Or(Or(a,b), Or(a,c))  =>  Or(a, Or(b,c))
    let a = SimpleValue::entry(vn_a());
    let b = SimpleValue::entry(vn_b());
    let c = SimpleValue::entry(vn_c());
    let left = SimpleValue::or(a.clone(), b.clone());
    let right = SimpleValue::or(a.clone(), c.clone());
    let result = SimpleValue::or(left, right).simplify();
    let outer = result.as_or().expect("expected outer Or");
    assert_eq!(outer.0.as_ref(), &a, "common factor should be left child");
    let inner = outer.1.as_ref().as_or().expect("expected inner Or");
    let inner_vals: Vec<_> = vec![inner.0.as_ref().clone(), inner.1.as_ref().clone()];
    assert!(inner_vals.contains(&b), "inner Or should contain b");
    assert!(inner_vals.contains(&c), "inner Or should contain c");
}

#[test]
fn or_canonical_or_on_right() {
    // Or(Or(a,b), c)  =>  non-Or on left, Or on right
    let a = SimpleValue::entry(vn_a());
    let b = SimpleValue::entry(vn_b());
    let c = SimpleValue::const_(42);
    let inner = SimpleValue::or(a, b);
    let result = SimpleValue::or(inner, c).simplify();
    let or = result.as_or().expect("expected Or");
    assert!(or.0.as_or().is_none(), "left child should not be an Or");
    assert!(or.1.as_or().is_some(), "right child should be an Or");
}

#[test]
fn or_variant_ordering() {
    // Or(entry, const)  =>  canonical form: const (rank 0) on left, entry (rank 1) on right
    let result = SimpleValue::or(SimpleValue::entry(vn_b()), SimpleValue::const_(7)).simplify();
    let or = result.as_or().expect("expected Or");
    assert!(
        or.0.as_const().is_some(),
        "lower-rank const should be on left"
    );
    assert!(
        or.1.as_entry().is_some(),
        "higher-rank entry should be on right"
    );
}

// --- XorExpr -----------------------------------------------------------------

#[test]
fn xor_const_folding() {
    let result = (SimpleValue::const_(0b1010) ^ SimpleValue::const_(0b1100)).simplify();
    assert_eq!(result, SimpleValue::make_const(0b0110, 8));
}

#[test]
fn xor_self() {
    let result = (SimpleValue::entry(vn_a()) ^ SimpleValue::entry(vn_a())).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 8));
}

#[test]
fn xor_identity_zero() {
    let result = (SimpleValue::entry(vn_a()) ^ SimpleValue::const_(0)).simplify();
    assert_eq!(result, SimpleValue::entry(vn_a()));
}

#[test]
fn xor_top_propagation() {
    let result = (SimpleValue::Top ^ SimpleValue::entry(vn_a())).simplify();
    assert_eq!(result, SimpleValue::Top);
}

#[test]
fn xor_normalizes_const_to_right() {
    let result = (SimpleValue::const_(5) ^ SimpleValue::entry(vn_a())).simplify();
    let xor = result.as_xor().expect("expected Xor node");
    assert!(xor.0.as_entry().is_some(), "expected entry on left");
    assert!(xor.1.as_const().is_some(), "expected const on right");
}

#[test]
fn xor_double_const() {
    let result = (SimpleValue::const_(0xFF) ^ SimpleValue::const_(0xFF)).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 8));
}

// --- AndExpr -----------------------------------------------------------------

#[test]
fn and_const_folding() {
    let result = (SimpleValue::const_(0b1010) & SimpleValue::const_(0b1100)).simplify();
    assert_eq!(result, SimpleValue::make_const(0b1000, 8));
}

#[test]
fn and_self() {
    let result = (SimpleValue::entry(vn_a()) & SimpleValue::entry(vn_a())).simplify();
    assert_eq!(result, SimpleValue::entry(vn_a()));
}

#[test]
fn and_zero() {
    let result = (SimpleValue::entry(vn_a()) & SimpleValue::const_(0)).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 8));
}

#[test]
fn and_all_ones_identity() {
    // entry of size 1 & 0xFF -> entry
    let vn = VarNode::new(0x100u64, 1u32, 0u32);
    let entry = SimpleValue::entry(vn);
    let result = (entry.clone() & SimpleValue::make_const(0xFF_i64, 1)).simplify();
    assert_eq!(result, entry);
}

#[test]
fn and_top_propagation() {
    let result = (SimpleValue::Top & SimpleValue::entry(vn_a())).simplify();
    assert_eq!(result, SimpleValue::Top);
}

#[test]
fn and_symbolic_stays_symbolic() {
    let result = (SimpleValue::entry(vn_a()) & SimpleValue::entry(vn_b())).simplify();
    assert!(result.as_and().is_some(), "expected And node");
}

#[test]
fn and_normalizes_const_to_right() {
    let result = (SimpleValue::const_(5) & SimpleValue::entry(vn_a())).simplify();
    let and = result.as_and().expect("expected And node");
    assert!(and.0.as_entry().is_some(), "expected entry on left");
    assert!(and.1.as_const().is_some(), "expected const on right");
}

// --- Load --------------------------------------------------------------------

#[test]
fn load_top_propagation() {
    let result = SimpleValue::load(SimpleValue::Top).simplify();
    assert_eq!(result, SimpleValue::Top);
}

#[test]
fn load_simplifies_child() {
    // load(5 + 3)  =>  Load(const_8, _)
    let child = SimpleValue::const_(5) + SimpleValue::const_(3);
    let result = SimpleValue::load(child).simplify();
    let load = result.as_load().expect("expected Load");
    assert_eq!(load.0.as_ref().as_const_value(), Some(8));
}

#[test]
fn load_preserves_size() {
    let child = SimpleValue::entry(vn_a());
    let node = SimpleValue::Load(Load(Intern::new(child), 4));
    let result = node.simplify();
    let load = result.as_load().expect("expected Load");
    assert_eq!(load.1, 4);
}

// --- IntEqual ----------------------------------------------------------------

#[test]
fn int_equal_const_folding_true() {
    let result = SimpleValue::int_equal(SimpleValue::const_(5), SimpleValue::const_(5)).simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_equal_const_folding_false() {
    let result = SimpleValue::int_equal(SimpleValue::const_(5), SimpleValue::const_(6)).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_equal_self() {
    let result =
        SimpleValue::int_equal(SimpleValue::entry(vn_a()), SimpleValue::entry(vn_a())).simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_equal_top() {
    let result = SimpleValue::int_equal(SimpleValue::Top, SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::Top);
}

// --- IntLess -----------------------------------------------------------------

#[test]
fn int_less_const_folding_true() {
    // unsigned: 3 < 5
    let result = SimpleValue::int_less(SimpleValue::const_(3), SimpleValue::const_(5)).simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_less_const_folding_false() {
    let result = SimpleValue::int_less(SimpleValue::const_(5), SimpleValue::const_(3)).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_less_self() {
    let result =
        SimpleValue::int_less(SimpleValue::entry(vn_a()), SimpleValue::entry(vn_a())).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_less_top() {
    let result = SimpleValue::int_less(SimpleValue::Top, SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::Top);
}

// --- IntSLess ----------------------------------------------------------------

#[test]
fn int_sless_const_folding_true() {
    // signed: -1 < 0
    let result = SimpleValue::int_sless(SimpleValue::const_(-1), SimpleValue::const_(0)).simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_sless_const_folding_false() {
    let result = SimpleValue::int_sless(SimpleValue::const_(5), SimpleValue::const_(3)).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_sless_self() {
    let result =
        SimpleValue::int_sless(SimpleValue::entry(vn_a()), SimpleValue::entry(vn_a())).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_sless_top() {
    let result = SimpleValue::int_sless(SimpleValue::Top, SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::Top);
}

// --- IntNotEqual -------------------------------------------------------------

#[test]
fn int_not_equal_const_folding_true() {
    let result =
        SimpleValue::int_not_equal(SimpleValue::const_(3), SimpleValue::const_(5)).simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_not_equal_const_folding_false() {
    let result =
        SimpleValue::int_not_equal(SimpleValue::const_(5), SimpleValue::const_(5)).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_not_equal_self() {
    let result = SimpleValue::int_not_equal(SimpleValue::entry(vn_a()), SimpleValue::entry(vn_a()))
        .simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_not_equal_top() {
    let result = SimpleValue::int_not_equal(SimpleValue::Top, SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::Top);
}

// --- IntLessEqual ------------------------------------------------------------

#[test]
fn int_less_equal_const_true_less() {
    let result =
        SimpleValue::int_less_equal(SimpleValue::const_(3), SimpleValue::const_(5)).simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_less_equal_const_true_equal() {
    let result =
        SimpleValue::int_less_equal(SimpleValue::const_(5), SimpleValue::const_(5)).simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_less_equal_self() {
    let result =
        SimpleValue::int_less_equal(SimpleValue::entry(vn_a()), SimpleValue::entry(vn_a()))
            .simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_less_equal_top() {
    let result = SimpleValue::int_less_equal(SimpleValue::Top, SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::Top);
}

// --- IntSLessEqual -----------------------------------------------------------

#[test]
fn int_sless_equal_const_true() {
    // signed: -1 <= 0
    let result =
        SimpleValue::int_sless_equal(SimpleValue::const_(-1), SimpleValue::const_(0)).simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_sless_equal_const_false() {
    let result =
        SimpleValue::int_sless_equal(SimpleValue::const_(5), SimpleValue::const_(3)).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_sless_equal_self() {
    let result =
        SimpleValue::int_sless_equal(SimpleValue::entry(vn_a()), SimpleValue::entry(vn_a()))
            .simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_sless_equal_top() {
    let result = SimpleValue::int_sless_equal(SimpleValue::Top, SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::Top);
}

// --- IntCarry ----------------------------------------------------------------

#[test]
fn int_carry_no_carry() {
    // 0x7F + 0x01 = 0x80, no 8-bit carry
    let result = SimpleValue::int_carry(
        SimpleValue::make_const(0x7F, 1),
        SimpleValue::make_const(0x01, 1),
    )
    .simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_carry_with_carry() {
    // 0xFF + 0x01 = 0x100, carry out
    let result = SimpleValue::int_carry(
        SimpleValue::make_const(0xFF, 1),
        SimpleValue::make_const(0x01, 1),
    )
    .simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_carry_64bit_overflow() {
    // u64::MAX + 1 overflows 64 bits
    let result = SimpleValue::int_carry(
        SimpleValue::make_const(u64::MAX as i64, 8),
        SimpleValue::make_const(1, 8),
    )
    .simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_carry_top() {
    let result = SimpleValue::int_carry(SimpleValue::Top, SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::Top);
}

// --- IntSCarry ---------------------------------------------------------------

#[test]
fn int_scarry_no_overflow() {
    // 1 + 1 = 2, no signed overflow for i8
    let result =
        SimpleValue::int_scarry(SimpleValue::make_const(1, 1), SimpleValue::make_const(1, 1))
            .simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_scarry_positive_overflow() {
    // i8::MAX + 1 overflows signed i8
    let result = SimpleValue::int_scarry(
        SimpleValue::make_const(0x7F, 1),
        SimpleValue::make_const(1, 1),
    )
    .simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_scarry_negative_no_overflow() {
    // (-1) + (-1) = -2, no i8 signed overflow
    let result = SimpleValue::int_scarry(
        SimpleValue::make_const(-1i8 as i64, 1),
        SimpleValue::make_const(-1i8 as i64, 1),
    )
    .simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_scarry_top() {
    let result = SimpleValue::int_scarry(SimpleValue::Top, SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::Top);
}

// --- IntSBorrow --------------------------------------------------------------

#[test]
fn int_sborrow_no_overflow() {
    // 1 - 0 = 1, no signed borrow
    let result =
        SimpleValue::int_sborrow(SimpleValue::make_const(1, 1), SimpleValue::make_const(0, 1))
            .simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_sborrow_self() {
    let result =
        SimpleValue::int_sborrow(SimpleValue::entry(vn_a()), SimpleValue::entry(vn_a())).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn int_sborrow_overflow() {
    // i8::MIN - 1 overflows signed subtraction
    let result = SimpleValue::int_sborrow(
        SimpleValue::make_const(i8::MIN as i64, 1),
        SimpleValue::make_const(1, 1),
    )
    .simplify();
    assert_eq!(result, SimpleValue::make_const(1, 1));
}

#[test]
fn int_sborrow_top() {
    let result = SimpleValue::int_sborrow(SimpleValue::Top, SimpleValue::const_(1)).simplify();
    assert_eq!(result, SimpleValue::Top);
}

// --- PopCount ----------------------------------------------------------------

#[test]
fn popcount_zero() {
    let result = SimpleValue::popcount(SimpleValue::const_(0)).simplify();
    assert_eq!(result, SimpleValue::make_const(0, 1));
}

#[test]
fn popcount_all_bits() {
    // make_const(-1, 8) -> all 64 bits set
    let result = SimpleValue::popcount(SimpleValue::make_const(-1, 8)).simplify();
    assert_eq!(result, SimpleValue::make_const(64, 1));
}

#[test]
fn popcount_top() {
    let result = SimpleValue::popcount(SimpleValue::Top).simplify();
    assert_eq!(result, SimpleValue::Top);
}

// --- SimpleValue dispatch ----------------------------------------------------

#[test]
fn leaf_values_unchanged() {
    assert_eq!(SimpleValue::const_(5).simplify(), SimpleValue::const_(5));
    assert_eq!(
        SimpleValue::entry(vn_a()).simplify(),
        SimpleValue::entry(vn_a())
    );
    assert_eq!(SimpleValue::Top.simplify(), SimpleValue::Top);
}

#[test]
fn dispatch_delegates_to_variant() {
    let expr = AddExpr(
        Intern::new(SimpleValue::entry(vn_a())),
        Intern::new(SimpleValue::const_(0)),
        8,
    );
    let via_variant = SimpleValue::Add(expr.clone()).simplify();
    let via_expr = expr.simplify();
    assert_eq!(via_variant, via_expr);
}
