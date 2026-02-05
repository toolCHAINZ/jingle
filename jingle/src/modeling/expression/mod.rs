use jingle_sleigh::PcodeOperation;
use z3::ast::BV;

/// Expresses the output (if one exists) of a pcode operation as a BV
/// in terms of the provided input BVs.
///
/// Assumes there are as many entries in the given input iterator as the given
/// operator requires; any extra entries will be ignored and missing entries
/// will cause this to yield `None`.
pub fn apply_to_bvs<I: Iterator<Item = BV>>(op: &PcodeOperation, args: I) -> Option<BV> {
    todo!()
}
