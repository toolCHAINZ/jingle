use crate::analysis::valuation::SimpleValuation;
use internment::Intern;
use jingle_sleigh::VarNode;

fn const_vn(val: u64, size: usize) -> VarNode {
    VarNode {
        space_index: VarNode::CONST_SPACE_INDEX,
        offset: val,
        size,
    }
}

fn entry_vn(offset: u64, size: usize) -> VarNode {
    VarNode {
        space_index: 1, // arbitrary non-const space
        offset,
        size,
    }
}

fn const_val(val: u64, size: usize) -> SimpleValuation {
    SimpleValuation::Const(Intern::new(const_vn(val, size)))
}

fn entry_val(offset: u64, size: usize) -> SimpleValuation {
    SimpleValuation::Entry(Intern::new(entry_vn(offset, size)))
}

#[test]
fn test_add_constant_positioning_and_fold() {
    // (#1 + x) -> (x + #1) and (#2 + #3) -> #5
    let x = entry_val(0, 8);
    let c1 = const_val(1, 8);
    let c2 = const_val(2, 8);
    let c3 = const_val(3, 8);

    // (#1 + x) simplifies to (x + #1)
    let expr = SimpleValuation::Add(Intern::new(c1.clone()), Intern::new(x.clone()));
    let simplified = expr.simplify();
    let expected = SimpleValuation::Add(Intern::new(x.clone()), Intern::new(c1.clone()));
    assert_eq!(simplified, expected);

    // (#2 + #3) -> #(5)
    let expr2 = SimpleValuation::Add(Intern::new(c2.clone()), Intern::new(c3.clone()));
    let simple2 = expr2.simplify();
    assert_eq!(simple2.as_const(), Some(5));
}

#[test]
fn test_nested_add_constant_fold() {
    // ((x + #2) + #3) -> (x + #5)
    let x = entry_val(7, 8);
    let c2 = const_val(2, 8);
    let c3 = const_val(3, 8);

    let inner = SimpleValuation::Add(Intern::new(x.clone()), Intern::new(c2.clone()));
    let outer = SimpleValuation::Add(Intern::new(inner), Intern::new(c3.clone()));
    let simple = outer.simplify();
    let expected = SimpleValuation::Add(Intern::new(x.clone()), Intern::new(const_val(5, 8)));
    assert_eq!(simple, expected);
}

#[test]
fn test_xor_x_x_zero() {
    let x = entry_val(2, 8);
    let xor = SimpleValuation::BitXor(Intern::new(x.clone()), Intern::new(x.clone()));
    let simple = xor.simplify();
    assert_eq!(simple.as_const(), Some(0));
}

#[test]
fn test_add_zero_identity() {
    let x = entry_val(3, 8);
    let zero = const_val(0, 8);
    let expr = SimpleValuation::Add(Intern::new(x.clone()), Intern::new(zero.clone()));
    let simple = expr.simplify();
    // identity removes constant
    assert_eq!(simple, x);
}

#[test]
fn test_mul_one() {
    let x = entry_val(4, 8);
    let one = const_val(1, 8);
    // (1 * x) -> (x * 1) -> x
    let expr = SimpleValuation::Mul(Intern::new(one.clone()), Intern::new(x.clone()));
    let simple = expr.simplify();
    assert_eq!(simple, x);
}
