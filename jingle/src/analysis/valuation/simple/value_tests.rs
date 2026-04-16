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
    let result = (Value::const_(3, 8) + Value::const_(4, 8)).simplify();
    assert_eq!(result, Value::make_const(7, 8));
}

#[test]
fn add_identity_zero() {
    let result = (Value::entry(vn_a()) + Value::const_(0, 8)).simplify();
    assert_eq!(result, Value::entry(vn_a()));
}

#[test]
fn add_top_left() {
    let result = (Value::Top + Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn add_top_right() {
    let result = (Value::const_(1, 8) + Value::Top).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn add_normalizes_const_to_right() {
    let result = (Value::const_(5, 8) + Value::entry(vn_a())).simplify();
    let add = result.as_add().expect("expected Add node");
    assert!(add.0.as_entry().is_some(), "expected entry on left");
    assert!(add.1.as_const().is_some(), "expected const on right");
}

#[test]
fn add_nested_const_folding() {
    let inner = Value::entry(vn_a()) + Value::const_(10, 8);
    let result = (inner + Value::const_(5, 8)).simplify();
    let add = result.as_add().expect("expected Add node");
    assert_eq!(add.0.as_ref(), &Value::entry(vn_a()));
    assert_eq!(add.1.as_ref().as_const_value(), Some(15));
}

#[test]
fn add_sub_interaction_positive() {
    let inner = Value::entry(vn_a()) - Value::const_(10, 8);
    let result = (inner + Value::const_(3, 8)).simplify();
    let sub = result.as_sub().expect("expected Sub node");
    assert_eq!(sub.0.as_ref(), &Value::entry(vn_a()));
    assert_eq!(sub.1.as_ref().as_const_value(), Some(7));
}

#[test]
fn add_sub_interaction_negative() {
    let inner = Value::entry(vn_a()) - Value::const_(3, 8);
    let result = (inner + Value::const_(10, 8)).simplify();
    let add = result.as_add().expect("expected Add node");
    assert_eq!(add.0.as_ref(), &Value::entry(vn_a()));
    assert_eq!(add.1.as_ref().as_const_value(), Some(7));
}

#[test]
fn add_wrapping_overflow() {
    let result = (Value::const_(i64::MAX, 8) + Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::make_const(i64::MIN, 8));
}

#[test]
fn add_negative_const() {
    let result = (Value::const_(-5, 8) + Value::const_(3, 8)).simplify();
    assert_eq!(result, Value::make_const(-2, 8));
}

#[test]
fn add_double_top() {
    let result = (Value::Top + Value::Top).simplify();
    assert_eq!(result, Value::Top);
}

// --- SubExpr -----------------------------------------------------------------

#[test]
fn sub_const_folding() {
    let result = (Value::const_(10, 8) - Value::const_(3, 8)).simplify();
    assert_eq!(result, Value::make_const(7, 8));
}

#[test]
fn sub_identity_zero() {
    let result = (Value::entry(vn_a()) - Value::const_(0, 8)).simplify();
    assert_eq!(result, Value::entry(vn_a()));
}

#[test]
fn sub_negative_const() {
    // entry - (-5)  =>  entry + 5
    let result = (Value::entry(vn_a()) - Value::const_(-5, 8)).simplify();
    let add = result.as_add().expect("expected Add node");
    assert_eq!(add.0.as_ref(), &Value::entry(vn_a()));
    assert_eq!(add.1.as_ref().as_const_value(), Some(5));
}

#[test]
fn sub_self() {
    let result = (Value::entry(vn_a()) - Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::make_const(0, 8));
}

#[test]
fn sub_top_left() {
    let result = (Value::Top - Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn sub_top_right() {
    let result = (Value::const_(1, 8) - Value::Top).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn sub_nested_add_const_positive() {
    // (entry + 10) - 3  =>  entry + 7
    let inner = Value::entry(vn_a()) + Value::const_(10, 8);
    let result = (inner - Value::const_(3, 8)).simplify();
    let add = result.as_add().expect("expected Add node");
    assert_eq!(add.0.as_ref(), &Value::entry(vn_a()));
    assert_eq!(add.1.as_ref().as_const_value(), Some(7));
}

#[test]
fn sub_nested_add_const_negative() {
    // (entry + 3) - 10  =>  entry - 7
    let inner = Value::entry(vn_a()) + Value::const_(3, 8);
    let result = (inner - Value::const_(10, 8)).simplify();
    let sub = result.as_sub().expect("expected Sub node");
    assert_eq!(sub.0.as_ref(), &Value::entry(vn_a()));
    assert_eq!(sub.1.as_ref().as_const_value(), Some(7));
}

#[test]
fn sub_nested_sub_const() {
    // (entry - 3) - 4  =>  entry - 7
    let inner = Value::entry(vn_a()) - Value::const_(3, 8);
    let result = (inner - Value::const_(4, 8)).simplify();
    let sub = result.as_sub().expect("expected Sub node");
    assert_eq!(sub.0.as_ref(), &Value::entry(vn_a()));
    assert_eq!(sub.1.as_ref().as_const_value(), Some(7));
}

#[test]
fn sub_wrapping_underflow() {
    let result = (Value::const_(i64::MIN, 8) - Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::make_const(i64::MAX, 8));
}

// --- MulExpr -----------------------------------------------------------------

#[test]
fn mul_const_folding() {
    let result = (Value::const_(6, 8) * Value::const_(7, 8)).simplify();
    assert_eq!(result, Value::make_const(42, 8));
}

#[test]
fn mul_identity_one() {
    let result = (Value::entry(vn_a()) * Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::entry(vn_a()));
}

#[test]
fn mul_zero() {
    let result = (Value::entry(vn_a()) * Value::const_(0, 8)).simplify();
    assert_eq!(result, Value::make_const(0, 8));
}

#[test]
fn mul_top_left() {
    let result = (Value::Top * Value::const_(5, 8)).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn mul_top_right() {
    let result = (Value::const_(5, 8) * Value::Top).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn mul_normalizes_const_to_right() {
    let result = (Value::const_(3, 8) * Value::entry(vn_a())).simplify();
    let mul = result.as_mul().expect("expected Mul node");
    assert!(mul.0.as_entry().is_some(), "expected entry on left");
    assert!(mul.1.as_const().is_some(), "expected const on right");
}

#[test]
fn mul_negative() {
    let result = (Value::const_(-3, 8) * Value::const_(4, 8)).simplify();
    assert_eq!(result, Value::make_const(-12, 8));
}

#[test]
fn mul_wrapping() {
    let result = (Value::const_(i64::MAX, 8) * Value::const_(2, 8)).simplify();
    assert_eq!(result, Value::make_const(i64::MAX.wrapping_mul(2), 8));
}

// --- Choice (abstract interpretation) ----------------------------------------

#[test]
fn or_identical_children() {
    let a = Value::entry(vn_a());
    let result = Value::choice(a.clone(), a).simplify();
    assert_eq!(result, Value::entry(vn_a()));
}

#[test]
fn or_top_left() {
    let a = Value::entry(vn_a());
    let result = Value::choice(Value::Top, a).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn or_top_right() {
    let a = Value::entry(vn_a());
    let result = Value::choice(a, Value::Top).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn or_nested_duplicate_inner_left() {
    // Choice(a, Choice(a, b))  =>  Choice(a, b)
    let a = Value::entry(vn_a());
    let b = Value::entry(vn_b());
    let inner = Value::choice(a.clone(), b.clone());
    let result = Value::choice(a.clone(), inner).simplify();
    let choice = result.as_choice().expect("expected Choice");
    assert_eq!(choice.0.as_ref(), &a);
    assert_eq!(choice.1.as_ref(), &b);
}

#[test]
fn or_nested_duplicate_inner_right() {
    // Choice(a, Choice(b, a))  =>  Choice(a, b)
    let a = Value::entry(vn_a());
    let b = Value::entry(vn_b());
    let inner = Value::choice(b.clone(), a.clone());
    let result = Value::choice(a.clone(), inner).simplify();
    let choice = result.as_choice().expect("expected Choice");
    assert_eq!(choice.0.as_ref(), &a);
    assert_eq!(choice.1.as_ref(), &b);
}

#[test]
fn or_common_factor_l1_r1() {
    // Choice(Choice(a,b), Choice(a,c))  =>  Choice(a, Choice(b,c))
    let a = Value::entry(vn_a());
    let b = Value::entry(vn_b());
    let c = Value::entry(vn_c());
    let left = Value::choice(a.clone(), b.clone());
    let right = Value::choice(a.clone(), c.clone());
    let result = Value::choice(left, right).simplify();
    let outer = result.as_choice().expect("expected outer Choice");
    assert_eq!(outer.0.as_ref(), &a, "common factor should be left child");
    let inner = outer.1.as_ref().as_choice().expect("expected inner Choice");
    let inner_vals: Vec<_> = vec![inner.0.as_ref().clone(), inner.1.as_ref().clone()];
    assert!(inner_vals.contains(&b), "inner Choice should contain b");
    assert!(inner_vals.contains(&c), "inner Choice should contain c");
}

#[test]
fn or_canonical_or_on_right() {
    // Choice(Choice(a,b), c)  =>  non-Choice on left, Choice on right
    let a = Value::entry(vn_a());
    let b = Value::entry(vn_b());
    let c = Value::const_(42, 8);
    let inner = Value::choice(a, b);
    let result = Value::choice(inner, c).simplify();
    let choice = result.as_choice().expect("expected Choice");
    assert!(
        choice.0.as_choice().is_none(),
        "left child should not be a Choice"
    );
    assert!(
        choice.1.as_choice().is_some(),
        "right child should be a Choice"
    );
}

#[test]
fn or_variant_ordering() {
    // Choice(entry, const)  =>  canonical form: const (rank 0) on left, entry (rank 1) on right
    let result = Value::choice(Value::entry(vn_b()), Value::const_(7, 8)).simplify();
    let choice = result.as_choice().expect("expected Choice");
    assert!(
        choice.0.as_const().is_some(),
        "lower-rank const should be on left"
    );
    assert!(
        choice.1.as_entry().is_some(),
        "higher-rank entry should be on right"
    );
}

// --- XorExpr -----------------------------------------------------------------

#[test]
fn xor_const_folding() {
    let result = (Value::const_(0b1010, 8) ^ Value::const_(0b1100, 8)).simplify();
    assert_eq!(result, Value::make_const(0b0110, 8));
}

#[test]
fn xor_self() {
    let result = (Value::entry(vn_a()) ^ Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::make_const(0, 8));
}

#[test]
fn xor_identity_zero() {
    let result = (Value::entry(vn_a()) ^ Value::const_(0, 8)).simplify();
    assert_eq!(result, Value::entry(vn_a()));
}

#[test]
fn xor_top_propagation() {
    let result = (Value::Top ^ Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn xor_normalizes_const_to_right() {
    let result = (Value::const_(5, 8) ^ Value::entry(vn_a())).simplify();
    let xor = result.as_xor().expect("expected Xor node");
    assert!(xor.0.as_entry().is_some(), "expected entry on left");
    assert!(xor.1.as_const().is_some(), "expected const on right");
}

#[test]
fn xor_double_const() {
    let result = (Value::const_(0xFF, 8) ^ Value::const_(0xFF, 8)).simplify();
    assert_eq!(result, Value::make_const(0, 8));
}

// --- AndExpr -----------------------------------------------------------------

#[test]
fn and_const_folding() {
    let result = (Value::const_(0b1010, 8) & Value::const_(0b1100, 8)).simplify();
    assert_eq!(result, Value::make_const(0b1000, 8));
}

#[test]
fn and_self() {
    let result = (Value::entry(vn_a()) & Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::entry(vn_a()));
}

#[test]
fn and_zero() {
    let result = (Value::entry(vn_a()) & Value::const_(0, 8)).simplify();
    assert_eq!(result, Value::make_const(0, 8));
}

#[test]
fn and_all_ones_identity() {
    // entry of size 1 & 0xFF -> entry
    let vn = VarNode::new(0x100u64, 1u32, 0u32);
    let entry = Value::entry(vn);
    let result = (entry.clone() & Value::make_const(0xFF_i64, 1)).simplify();
    assert_eq!(result, entry);
}

#[test]
fn and_top_propagation() {
    let result = (Value::Top & Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn and_symbolic_stays_symbolic() {
    let result = (Value::entry(vn_a()) & Value::entry(vn_b())).simplify();
    assert!(result.as_and().is_some(), "expected And node");
}

#[test]
fn and_normalizes_const_to_right() {
    let result = (Value::const_(5, 8) & Value::entry(vn_a())).simplify();
    let and = result.as_and().expect("expected And node");
    assert!(and.0.as_entry().is_some(), "expected entry on left");
    assert!(and.1.as_const().is_some(), "expected const on right");
}

// --- OrExpr (bitwise OR) -----------------------------------------------------

#[test]
fn or_bitwise_const_folding() {
    let result = (Value::const_(0b1010, 8) | Value::const_(0b1100, 8)).simplify();
    assert_eq!(result, Value::make_const(0b1110, 8));
}

#[test]
fn or_bitwise_self() {
    let result = (Value::entry(vn_a()) | Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::entry(vn_a()));
}

#[test]
fn or_bitwise_identity_zero() {
    let result = (Value::entry(vn_a()) | Value::const_(0, 8)).simplify();
    assert_eq!(result, Value::entry(vn_a()));
}

#[test]
fn or_bitwise_all_ones() {
    // entry | 0xFF -> 0xFF (for 1-byte values)
    let vn = VarNode::new(0x100u64, 1u32, 0u32);
    let entry = Value::entry(vn);
    let result = (entry | Value::make_const(0xFF_i64, 1)).simplify();
    assert_eq!(result, Value::make_const(0xFF, 1));
}

#[test]
fn or_bitwise_top_propagation() {
    let result = (Value::Top | Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn or_bitwise_symbolic_stays_symbolic() {
    let result = (Value::entry(vn_a()) | Value::entry(vn_b())).simplify();
    assert!(result.as_or().is_some(), "expected Or node");
}

#[test]
fn or_bitwise_normalizes_const_to_right() {
    let result = (Value::const_(5, 8) | Value::entry(vn_a())).simplify();
    let or = result.as_or().expect("expected Or node");
    assert!(or.0.as_entry().is_some(), "expected entry on left");
    assert!(or.1.as_const().is_some(), "expected const on right");
}

// --- BoolExpr ----------------------------------------------------------------

#[test]
fn bool_negate_const_zero() {
    let result = Value::bool_negate(Value::const_(0, 1)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn bool_negate_const_one() {
    let result = Value::bool_negate(Value::const_(1, 1)).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn bool_negate_nonzero_const_is_false() {
    let result = Value::bool_negate(Value::const_(5, 1)).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn bool_negate_double_negation_on_boolean_value() {
    let flag = Value::int_equal(Value::entry(vn_a()), Value::entry(vn_b()));
    let inner = Value::bool_negate(flag.clone());
    let result = Value::bool_negate(inner).simplify();
    assert_eq!(result, flag.simplify());
}

#[test]
fn bool_and_const_folding() {
    let result = Value::bool_and(Value::const_(1, 1), Value::const_(5, 1)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn bool_and_zero_short_circuit() {
    let result = Value::bool_and(Value::entry(vn_a()), Value::const_(0, 1)).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn bool_and_true_identity_for_boolean_value() {
    let flag = Value::int_equal(Value::entry(vn_a()), Value::entry(vn_b()));
    let result = Value::bool_and(flag.clone(), Value::const_(1, 1)).simplify();
    assert_eq!(result, flag.simplify());
}

#[test]
fn bool_or_const_folding() {
    let result = Value::bool_or(Value::const_(0, 1), Value::const_(2, 1)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn bool_or_false_identity_for_boolean_value() {
    let flag = Value::int_less(Value::entry(vn_a()), Value::entry(vn_b()));
    let result = Value::bool_or(flag.clone(), Value::const_(0, 1)).simplify();
    assert_eq!(result, flag.simplify());
}

#[test]
fn bool_or_true_short_circuit() {
    let result = Value::bool_or(Value::entry(vn_a()), Value::const_(1, 1)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn bool_xor_const_folding() {
    let result = Value::bool_xor(Value::const_(1, 1), Value::const_(0, 1)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn bool_xor_self_boolean_value_is_false() {
    let flag = Value::int_equal(Value::entry(vn_a()), Value::entry(vn_b()));
    let result = Value::bool_xor(flag.clone(), flag).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn bool_xor_true_flips_boolean_value() {
    let flag = Value::int_equal(Value::entry(vn_a()), Value::entry(vn_b()));
    let result = Value::bool_xor(flag.clone(), Value::const_(1, 1)).simplify();
    let expr = result.as_bool_negate().expect("expected BoolNegate node");
    assert_eq!(expr.0.as_ref(), &flag.simplify());
}

#[test]
fn bool_ops_top_propagate() {
    assert_eq!(Value::bool_negate(Value::Top).simplify(), Value::Top);
    assert_eq!(
        Value::bool_and(Value::Top, Value::const_(1, 1)).simplify(),
        Value::Top
    );
    assert_eq!(
        Value::bool_or(Value::Top, Value::const_(0, 1)).simplify(),
        Value::Top
    );
    assert_eq!(
        Value::bool_xor(Value::Top, Value::const_(1, 1)).simplify(),
        Value::Top
    );
}

#[test]
fn substitute_bool_nodes_and_simplifies() {
    let mut context = crate::analysis::valuation::simple::valuation::ValuationSet::new();
    context.direct_writes.insert(vn_a(), Value::const_(0, 1));
    let expr = Value::bool_negate(Value::entry(vn_a()));

    assert_eq!(expr.substitute(&context), Value::const_(1, 1));
}

// --- Load --------------------------------------------------------------------

#[test]
fn load_top_propagation() {
    let result = Value::load(Value::Top, 8).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn load_simplifies_child() {
    // load(5 + 3)  =>  Load(const_8, _)
    let child = Value::const_(5, 8) + Value::const_(3, 8);
    let result = Value::load(child, 8).simplify();
    let load = result.as_load().expect("expected Load");
    assert_eq!(load.0.as_ref().as_const_value(), Some(8));
}

#[test]
fn load_preserves_size() {
    let child = Value::entry(vn_a());
    let node = Value::Load(Load(Intern::new(child), 4));
    let result = node.simplify();
    let load = result.as_load().expect("expected Load");
    assert_eq!(load.1, 4);
}

// --- IntEqual ----------------------------------------------------------------

#[test]
fn int_equal_const_folding_true() {
    let result = Value::int_equal(Value::const_(5, 8), Value::const_(5, 8)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_equal_const_folding_false() {
    let result = Value::int_equal(Value::const_(5, 8), Value::const_(6, 8)).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_equal_self() {
    let result = Value::int_equal(Value::entry(vn_a()), Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_equal_top() {
    let result = Value::int_equal(Value::Top, Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::Top);
}

// --- IntLess -----------------------------------------------------------------

#[test]
fn int_less_const_folding_true() {
    // unsigned: 3 < 5
    let result = Value::int_less(Value::const_(3, 8), Value::const_(5, 8)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_less_const_folding_false() {
    let result = Value::int_less(Value::const_(5, 8), Value::const_(3, 8)).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_less_self() {
    let result = Value::int_less(Value::entry(vn_a()), Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_less_top() {
    let result = Value::int_less(Value::Top, Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::Top);
}

// --- IntSLess ----------------------------------------------------------------

#[test]
fn int_sless_const_folding_true() {
    // signed: -1 < 0
    let result = Value::int_sless(Value::const_(-1, 8), Value::const_(0, 8)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_sless_const_folding_false() {
    let result = Value::int_sless(Value::const_(5, 8), Value::const_(3, 8)).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_sless_self() {
    let result = Value::int_sless(Value::entry(vn_a()), Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_sless_top() {
    let result = Value::int_sless(Value::Top, Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::Top);
}

// --- IntNotEqual -------------------------------------------------------------

#[test]
fn int_not_equal_const_folding_true() {
    let result = Value::int_not_equal(Value::const_(3, 8), Value::const_(5, 8)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_not_equal_const_folding_false() {
    let result = Value::int_not_equal(Value::const_(5, 8), Value::const_(5, 8)).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_not_equal_self() {
    let result = Value::int_not_equal(Value::entry(vn_a()), Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_not_equal_top() {
    let result = Value::int_not_equal(Value::Top, Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::Top);
}

// --- IntLessEqual ------------------------------------------------------------

#[test]
fn int_less_equal_const_true_less() {
    let result = Value::int_less_equal(Value::const_(3, 8), Value::const_(5, 8)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_less_equal_const_true_equal() {
    let result = Value::int_less_equal(Value::const_(5, 8), Value::const_(5, 8)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_less_equal_self() {
    let result = Value::int_less_equal(Value::entry(vn_a()), Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_less_equal_top() {
    let result = Value::int_less_equal(Value::Top, Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::Top);
}

// --- IntSLessEqual -----------------------------------------------------------

#[test]
fn int_sless_equal_const_true() {
    // signed: -1 <= 0
    let result = Value::int_sless_equal(Value::const_(-1, 8), Value::const_(0, 8)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_sless_equal_const_false() {
    let result = Value::int_sless_equal(Value::const_(5, 8), Value::const_(3, 8)).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_sless_equal_self() {
    let result = Value::int_sless_equal(Value::entry(vn_a()), Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_sless_equal_top() {
    let result = Value::int_sless_equal(Value::Top, Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::Top);
}

// --- IntCarry ----------------------------------------------------------------

#[test]
fn int_carry_no_carry() {
    // 0x7F + 0x01 = 0x80, no 8-bit carry
    let result =
        Value::int_carry(Value::make_const(0x7F, 1), Value::make_const(0x01, 1)).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_carry_with_carry() {
    // 0xFF + 0x01 = 0x100, carry out
    let result =
        Value::int_carry(Value::make_const(0xFF, 1), Value::make_const(0x01, 1)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_carry_64bit_overflow() {
    // u64::MAX + 1 overflows 64 bits
    let result = Value::int_carry(
        Value::make_const(u64::MAX as i64, 8),
        Value::make_const(1, 8),
    )
    .simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_carry_top() {
    let result = Value::int_carry(Value::Top, Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::Top);
}

// --- IntSCarry ---------------------------------------------------------------

#[test]
fn int_scarry_no_overflow() {
    // 1 + 1 = 2, no signed overflow for i8
    let result = Value::int_scarry(Value::make_const(1, 1), Value::make_const(1, 1)).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_scarry_positive_overflow() {
    // i8::MAX + 1 overflows signed i8
    let result = Value::int_scarry(Value::make_const(0x7F, 1), Value::make_const(1, 1)).simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_scarry_negative_no_overflow() {
    // (-1) + (-1) = -2, no i8 signed overflow
    let result = Value::int_scarry(
        Value::make_const(-1i8 as i64, 1),
        Value::make_const(-1i8 as i64, 1),
    )
    .simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_scarry_top() {
    let result = Value::int_scarry(Value::Top, Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::Top);
}

// --- IntSBorrow --------------------------------------------------------------

#[test]
fn int_sborrow_no_overflow() {
    // 1 - 0 = 1, no signed borrow
    let result = Value::int_sborrow(Value::make_const(1, 1), Value::make_const(0, 1)).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_sborrow_self() {
    let result = Value::int_sborrow(Value::entry(vn_a()), Value::entry(vn_a())).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn int_sborrow_overflow() {
    // i8::MIN - 1 overflows signed subtraction
    let result = Value::int_sborrow(
        Value::make_const(i8::MIN as i64, 1),
        Value::make_const(1, 1),
    )
    .simplify();
    assert_eq!(result, Value::make_const(1, 1));
}

#[test]
fn int_sborrow_top() {
    let result = Value::int_sborrow(Value::Top, Value::const_(1, 8)).simplify();
    assert_eq!(result, Value::Top);
}

// --- PopCount ----------------------------------------------------------------

#[test]
fn popcount_zero() {
    let result = Value::popcount(Value::const_(0, 8)).simplify();
    assert_eq!(result, Value::make_const(0, 1));
}

#[test]
fn popcount_all_bits() {
    // make_const(-1, 8) -> all 64 bits set
    let result = Value::popcount(Value::make_const(-1, 8)).simplify();
    assert_eq!(result, Value::make_const(64, 1));
}

#[test]
fn popcount_top() {
    let result = Value::popcount(Value::Top).simplify();
    assert_eq!(result, Value::Top);
}

// --- Value dispatch ----------------------------------------------------

#[test]
fn leaf_values_unchanged() {
    assert_eq!(Value::const_(5, 8).simplify(), Value::const_(5, 8));
    assert_eq!(Value::entry(vn_a()).simplify(), Value::entry(vn_a()));
    assert_eq!(Value::Top.simplify(), Value::Top);
}

#[test]
fn dispatch_delegates_to_variant() {
    let expr = AddExpr(
        Intern::new(Value::entry(vn_a())),
        Intern::new(Value::const_(0, 8)),
        8,
    );
    let via_variant = Value::Add(expr.clone()).simplify();
    let via_expr = expr.simplify();
    assert_eq!(via_variant, via_expr);
}

// --- ZeroExtend --------------------------------------------------------------

#[test]
fn zero_extend_const_folding() {
    // zext(0x42 as u8, 4) → 0x00000042 as u32
    let inner = Value::make_const(0x42, 1);
    let result = Value::zero_extend(inner, 4).simplify();
    assert_eq!(result, Value::make_const(0x42, 4));
}

#[test]
fn zero_extend_const_masks_high_bits() {
    // zext(-1 as i8, 4) — -1 in i64 has all bits set, but as a 1-byte const
    // the stored offset is 0xFFFFFFFFFFFFFFFF. We should mask to 0x000000FF.
    let inner = Value::make_const(-1, 1);
    let result = Value::zero_extend(inner, 4).simplify();
    assert_eq!(result, Value::make_const(0xFF, 4));
}

#[test]
fn zero_extend_identity_same_size() {
    let entry = Value::entry(vn_a());
    let result = Value::zero_extend(entry.clone(), entry.size()).simplify();
    assert_eq!(result, entry);
}

#[test]
fn zero_extend_top() {
    let result = Value::zero_extend(Value::Top, 8).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn zero_extend_chain_collapses() {
    // zext(zext(x, 4), 8) → zext(x, 8)
    let entry = Value::entry(vn_a());
    let inner = Value::zero_extend(entry.clone(), 4);
    let result = Value::zero_extend(inner, 8).simplify();
    assert_eq!(result, Value::zero_extend(entry, 8).simplify());
}

// --- SignExtend --------------------------------------------------------------

#[test]
fn sign_extend_const_positive() {
    // sext(0x42 as u8, 4) — sign bit of byte 0x42 is 0, so result is 0x00000042
    let inner = Value::make_const(0x42, 1);
    let result = Value::sign_extend(inner, 4).simplify();
    assert_eq!(result, Value::make_const(0x42, 4));
}

#[test]
fn sign_extend_const_negative() {
    // sext(0xFF as u8, 4) — sign bit is 1, so result is -1 as i32 = 0xFFFFFFFF
    let inner = Value::make_const(0xFF, 1);
    let result = Value::sign_extend(inner, 4).simplify();
    // 0xFFFFFFFF as i64 = 4294967295, but as i32 (4-byte) = -1
    assert_eq!(result, Value::make_const(0xFFFF_FFFF_u64 as i64, 4));
}

#[test]
fn sign_extend_identity_same_size() {
    let entry = Value::entry(vn_a());
    let result = Value::sign_extend(entry.clone(), entry.size()).simplify();
    assert_eq!(result, entry);
}

#[test]
fn sign_extend_top() {
    let result = Value::sign_extend(Value::Top, 8).simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn sign_extend_chain_collapses() {
    // sext(sext(x, 4), 8) → sext(x, 8)
    let entry = Value::entry(vn_a());
    let inner = Value::sign_extend(entry.clone(), 4);
    let result = Value::sign_extend(inner, 8).simplify();
    assert_eq!(result, Value::sign_extend(entry, 8).simplify());
}

// --- Extract -----------------------------------------------------------------

#[test]
fn extract_const_folding() {
    // extract(0xAABBCCDD as u32, byte_offset=1, output_size=1) → 0xCC
    let inner = Value::make_const(0xAABBCCDD_u64 as i64, 4);
    let result = Value::extract(inner, 1, 1).simplify();
    assert_eq!(result, Value::make_const(0xCC, 1));
}

#[test]
fn extract_identity_full() {
    // extract(x, 0, x.size()) → x
    let entry = Value::entry(vn_a());
    let size = entry.size();
    let result = Value::extract(entry.clone(), 0, size).simplify();
    assert_eq!(result, entry);
}

#[test]
fn extract_top() {
    let result = Value::extract(Value::Top, 0, 4).simplify();
    assert_eq!(result, Value::Top);
}

// --- Trimming via Valuation::add ---------------------------------------

#[test]
fn add_trims_covered_sub_entries() {
    use crate::analysis::valuation::simple::valuation::ValuationSet;

    let mut val = ValuationSet::new();

    // Write a 4-byte entry at offset 4 in space 0
    let small_vn = VarNode::new(4u64, 4u32, 0u32);
    val.add(small_vn, Value::const_(0x42, 8));

    // Now write an 8-byte entry at offset 4 in the same space — covers small_vn
    let big_vn = VarNode::new(4u64, 8u32, 0u32);
    val.add(big_vn, Value::const_(0x1234, 8));

    // The small entry should have been removed
    assert!(
        val.direct_writes.get(small_vn).is_none(),
        "small entry should be trimmed after larger write covers it"
    );
    // The large entry should still be present
    assert!(val.direct_writes.get(big_vn).is_some());
}

#[test]
fn add_does_not_trim_non_covered_entries() {
    use crate::analysis::valuation::simple::valuation::ValuationSet;

    let mut val = ValuationSet::new();

    // Write entries at different offsets in space 0
    let vn_other = VarNode::new(0x100u64, 4u32, 0u32);
    val.add(vn_other, Value::const_(0x99, 8));

    // Write at a completely different offset — should not trim vn_other
    let vn_new = VarNode::new(4u64, 8u32, 0u32);
    val.add(vn_new, Value::const_(0x1234, 8));

    assert!(
        val.direct_writes.get(vn_other).is_some(),
        "unrelated entry should not be trimmed"
    );
}

// --- IntLeftShift ------------------------------------------------------------

#[test]
fn left_shift_const_folding() {
    // 8 << 2 = 32
    let result = Value::IntLeftShift(IntLeftShiftExpr(
        Intern::new(Value::const_(8, 8)),
        Intern::new(Value::const_(2, 8)),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::make_const(32, 8));
}

#[test]
fn left_shift_identity_zero() {
    // expr << 0 = expr
    let result = Value::IntLeftShift(IntLeftShiftExpr(
        Intern::new(Value::entry(vn_a())),
        Intern::new(Value::const_(0, 8)),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::entry(vn_a()));
}

#[test]
fn left_shift_overflow() {
    // Shifting by >= bit width should return 0
    let result = Value::IntLeftShift(IntLeftShiftExpr(
        Intern::new(Value::const_(0xFF, 8)),
        Intern::new(Value::const_(64, 8)), // 8 bytes * 8 = 64 bits
        8,
    ))
    .simplify();
    assert_eq!(result, Value::make_const(0, 8));
}

#[test]
fn left_shift_overflow_beyond_size() {
    // Shifting by more than bit width
    let result = Value::IntLeftShift(IntLeftShiftExpr(
        Intern::new(Value::const_(0xFF, 8)),
        Intern::new(Value::const_(100, 8)),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::make_const(0, 8));
}

#[test]
fn left_shift_with_masking() {
    // 0xFF << 4 should properly mask overflow bits
    let result = Value::IntLeftShift(IntLeftShiftExpr(
        Intern::new(Value::make_const(0xFF, 1)), // 1 byte
        Intern::new(Value::const_(4, 8)),
        1,
    ))
    .simplify();
    // 0xFF << 4 = 0xFF0, masked to 1 byte = 0xF0
    assert_eq!(result, Value::make_const(0xF0, 1));
}

#[test]
fn left_shift_top_left() {
    let result = Value::IntLeftShift(IntLeftShiftExpr(
        Intern::new(Value::Top),
        Intern::new(Value::const_(2, 8)),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn left_shift_top_right() {
    let result = Value::IntLeftShift(IntLeftShiftExpr(
        Intern::new(Value::const_(8, 8)),
        Intern::new(Value::Top),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn left_shift_symbolic() {
    // Non-constant shift should not fold
    let result = Value::IntLeftShift(IntLeftShiftExpr(
        Intern::new(Value::entry(vn_a())),
        Intern::new(Value::const_(2, 8)),
        8,
    ))
    .simplify();
    assert!(result.as_add().is_none()); // Should NOT be addition
    // Should remain a left shift operation (not simplified to a different form)
    let expr = Value::IntLeftShift(IntLeftShiftExpr(
        Intern::new(Value::entry(vn_a())),
        Intern::new(Value::const_(2, 8)),
        8,
    ));
    assert_eq!(result, expr);
}

// --- IntRightShift (unsigned/logical) ----------------------------------------

#[test]
fn right_shift_const_folding() {
    // 32 >> 2 = 8
    let result = Value::IntRightShift(IntRightShiftExpr(
        Intern::new(Value::const_(32, 8)),
        Intern::new(Value::const_(2, 8)),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::make_const(8, 8));
}

#[test]
fn right_shift_identity_zero() {
    // expr >> 0 = expr
    let result = Value::IntRightShift(IntRightShiftExpr(
        Intern::new(Value::entry(vn_a())),
        Intern::new(Value::const_(0, 8)),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::entry(vn_a()));
}

#[test]
fn right_shift_overflow() {
    // Shifting by >= bit width should return 0
    let result = Value::IntRightShift(IntRightShiftExpr(
        Intern::new(Value::const_(0xFF, 8)),
        Intern::new(Value::const_(64, 8)), // 8 bytes * 8 = 64 bits
        8,
    ))
    .simplify();
    assert_eq!(result, Value::make_const(0, 8));
}

#[test]
fn right_shift_fills_with_zeros() {
    // Unsigned right shift fills with zeros (logical shift)
    // 0xF0 >> 4 = 0x0F
    let result = Value::IntRightShift(IntRightShiftExpr(
        Intern::new(Value::make_const(0xF0, 1)),
        Intern::new(Value::const_(4, 8)),
        1,
    ))
    .simplify();
    assert_eq!(result, Value::make_const(0x0F, 1));
}

#[test]
fn right_shift_top_left() {
    let result = Value::IntRightShift(IntRightShiftExpr(
        Intern::new(Value::Top),
        Intern::new(Value::const_(2, 8)),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn right_shift_top_right() {
    let result = Value::IntRightShift(IntRightShiftExpr(
        Intern::new(Value::const_(8, 8)),
        Intern::new(Value::Top),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::Top);
}

// --- IntSignedRightShift (arithmetic) ----------------------------------------

#[test]
fn signed_right_shift_const_folding_positive() {
    // 32 s>> 2 = 8 (positive value)
    let result = Value::IntSignedRightShift(IntSignedRightShiftExpr(
        Intern::new(Value::const_(32, 8)),
        Intern::new(Value::const_(2, 8)),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::make_const(8, 8));
}

#[test]
fn signed_right_shift_const_folding_negative() {
    // For a 1-byte value: -16 (0xF0) s>> 2 should preserve sign
    // 0xF0 = binary 11110000, arithmetic shift right by 2 = 11111100 = 0xFC = -4
    let result = Value::IntSignedRightShift(IntSignedRightShiftExpr(
        Intern::new(Value::make_const(0xF0u64 as i64, 1)),
        Intern::new(Value::const_(2, 8)),
        1,
    ))
    .simplify();
    assert_eq!(result, Value::make_const(0xFCu64 as i64, 1));
}

#[test]
fn signed_right_shift_identity_zero() {
    // expr s>> 0 = expr
    let result = Value::IntSignedRightShift(IntSignedRightShiftExpr(
        Intern::new(Value::entry(vn_a())),
        Intern::new(Value::const_(0, 8)),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::entry(vn_a()));
}

#[test]
fn signed_right_shift_overflow_positive() {
    // Positive value shifted >= bit width should return 0
    let result = Value::IntSignedRightShift(IntSignedRightShiftExpr(
        Intern::new(Value::const_(42, 8)),
        Intern::new(Value::const_(64, 8)),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::make_const(0, 8));
}

#[test]
fn signed_right_shift_overflow_negative() {
    // Negative value shifted >= bit width should return -1
    // For 1-byte: 0x80 is -128, shift by 8 or more should give -1 (0xFF)
    let result = Value::IntSignedRightShift(IntSignedRightShiftExpr(
        Intern::new(Value::make_const(0x80u64 as i64, 1)),
        Intern::new(Value::const_(8, 8)),
        1,
    ))
    .simplify();
    assert_eq!(result, Value::make_const(-1, 1));
}

#[test]
fn signed_right_shift_preserves_sign_bit() {
    // -1 (all bits set) s>> 4 should still be -1
    let result = Value::IntSignedRightShift(IntSignedRightShiftExpr(
        Intern::new(Value::make_const(0xFFu64 as i64, 1)),
        Intern::new(Value::const_(4, 8)),
        1,
    ))
    .simplify();
    assert_eq!(result, Value::make_const(0xFFu64 as i64, 1));
}

#[test]
fn signed_right_shift_top_left() {
    let result = Value::IntSignedRightShift(IntSignedRightShiftExpr(
        Intern::new(Value::Top),
        Intern::new(Value::const_(2, 8)),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn signed_right_shift_top_right() {
    let result = Value::IntSignedRightShift(IntSignedRightShiftExpr(
        Intern::new(Value::const_(8, 8)),
        Intern::new(Value::Top),
        8,
    ))
    .simplify();
    assert_eq!(result, Value::Top);
}

#[test]
fn signed_right_shift_small_negative() {
    // Test a small negative number: -2 (0xFE in 1 byte) s>> 1 = -1 (0xFF)
    let result = Value::IntSignedRightShift(IntSignedRightShiftExpr(
        Intern::new(Value::make_const(0xFEu64 as i64, 1)),
        Intern::new(Value::const_(1, 8)),
        1,
    ))
    .simplify();
    assert_eq!(result, Value::make_const(0xFFu64 as i64, 1));
}
