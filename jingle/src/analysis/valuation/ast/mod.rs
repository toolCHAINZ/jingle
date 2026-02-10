use internment::Intern;
use jingle_sleigh::VarNode;

trait Simplify {
    fn simplify(&self) -> SimpleValue;
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Entry(Intern<VarNode>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Mul(Intern<SimpleValue>, Intern<SimpleValue>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Add(Intern<SimpleValue>, Intern<SimpleValue>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Sub(Intern<SimpleValue>, Intern<SimpleValue>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Or(Intern<SimpleValue>, Intern<SimpleValue>);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Load(Intern<SimpleValue>);

/// Symbolic valuation built from varnodes and constants.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum SimpleValue {
    Entry(Entry),
    Const(i64),

    // Binary operators now use interned children (via `internment`) rather than Arc'd tuples.
    Mul(Mul),
    Add(Add),
    Sub(Sub),

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

impl Simplify for Add {
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
                    let sub = Sub(
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
        if let SimpleValue::Add(Add(left_inner_left, left_inner_right)) = &left {
            if let SimpleValue::Const(inner_right_const) = left_inner_right.as_ref() {
                if let SimpleValue::Const(right_const) = &right {
                    let res = inner_right_const.wrapping_add(*right_const);
                    let new_const = SimpleValue::Const(res);
                    return Add(left_inner_left.clone(), Intern::new(new_const)).simplify();
                }
            }
        }

        // ((expr - #a) + #b) -> (expr - #(a - b))
        if let SimpleValue::Sub(Sub(expr, a)) = &left {
            if let SimpleValue::Const(a_const) = a.as_ref() {
                if let SimpleValue::Const(b) = &right {
                    let res = a_const.wrapping_sub(*b);
                    let new_const = SimpleValue::Const(res);
                    return Sub(expr.clone(), Intern::new(new_const)).simplify();
                }
            }
        }

        // default: rebuild with simplified children
        SimpleValue::Add(Add(Intern::new(left), Intern::new(right)))
    }
}

impl Simplify for Sub {
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
                    let add = Add(
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
        if let SimpleValue::Add(Add(expr, a)) = &left {
            if let SimpleValue::Const(a) = a.as_ref() {
                if let SimpleValue::Const(b) = &right {
                    let res = a.wrapping_sub(*b);
                    let new_const = SimpleValue::Const(res);
                    return Add(expr.clone(), Intern::new(new_const)).simplify();
                }
            }
        }

        // ((expr - #a) - #b) -> (expr - #(a + b))
        if let SimpleValue::Sub(Sub(expr, a)) = &left {
            if let SimpleValue::Const(a) = a.as_ref() {
                if let SimpleValue::Const(b) = &right {
                    let res = a.wrapping_add(*b);
                    let new_const = SimpleValue::Const(res);
                    return Sub(expr.clone(), Intern::new(new_const)).simplify();
                }
            }
        }

        SimpleValue::Sub(Sub(Intern::new(left), Intern::new(right)))
    }
}

impl Simplify for Mul {
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

        SimpleValue::Mul(Mul(Intern::new(left), Intern::new(right)))
    }
}

impl Simplify for Or {
    fn simplify(&self) -> SimpleValue {
        let a_intern = self.0;
        let b_intern = self.1;

        let a_s = a_intern.as_ref().simplify();
        let b_s = b_intern.as_ref().simplify();

        if matches!(a_s, SimpleValue::Top) || matches!(b_s, SimpleValue::Top) {
            return SimpleValue::Top;
        }

        if a_s == b_s {
            return a_s;
        }

        SimpleValue::Or(Or(Intern::new(a_s), Intern::new(b_s)))
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
