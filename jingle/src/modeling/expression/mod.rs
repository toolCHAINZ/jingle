use jingle_sleigh::PcodeOperation;
use std::cmp::{Ordering, min};
use z3::ast::BV;

/// Expresses the output (if one exists) of a pcode operation as a BV
/// in terms of the provided input BVs.
///
/// Assumes there are as many entries in the given input iterator as the given
/// operator requires; any extra entries will be ignored and missing entries
/// will cause this to yield `None`.
///
/// Does not model Load or Store, only operations on direct data
pub fn apply_to_bvs<I: Iterator<Item = BV>>(op: &PcodeOperation, args: I) -> Option<BV> {
    let vals: Vec<BV> = args.collect();

    // helper to get a ref to arg i
    let arg = |i: usize| -> Option<&BV> { vals.get(i) };

    match op {
        PcodeOperation::Copy { .. } => arg(0).cloned(),
        PcodeOperation::IntZExt { input, output } => {
            arg(0).map(|v| v.zero_ext(((output.size - input.size) as u32) * 8))
        }
        PcodeOperation::IntSExt { input, output } => {
            arg(0).map(|v| v.sign_ext(((output.size - input.size) as u32) * 8))
        }
        PcodeOperation::Store { .. } => {
            // store has no BV output
            None
        }
        PcodeOperation::Load { .. } => {
            // Not modeled
            None
        }
        PcodeOperation::IntAdd { .. } => Some(arg(0)?.bvadd(arg(1)?)),
        PcodeOperation::IntSub { .. } => Some(arg(0)? - arg(1)?),
        PcodeOperation::IntAnd { .. } => Some(arg(0)?.bvand(arg(1)?)),
        PcodeOperation::IntXor { .. } => Some(arg(0)?.bvxor(arg(1)?)),
        PcodeOperation::IntOr { .. } => Some(arg(0)?.bvor(arg(1)?)),
        PcodeOperation::IntNegate { .. } => Some(arg(0)?.bvneg()),
        PcodeOperation::IntMult { .. } => Some(arg(0)?.bvmul(arg(1)?)),
        PcodeOperation::IntDiv { .. } => Some(arg(0)?.bvudiv(arg(1)?)),
        PcodeOperation::IntSignedDiv { .. } => Some(arg(0)?.bvsdiv(arg(1)?)),
        PcodeOperation::IntRem { .. } => Some(arg(0)?.bvurem(arg(1)?)),
        PcodeOperation::IntSignedRem { .. } => Some(arg(0)?.bvsrem(arg(1)?)),
        PcodeOperation::IntRightShift { .. } => {
            let bv1 = arg(0)?;
            let mut bv2 = arg(1)?.clone();
            match bv1.get_size().cmp(&bv2.get_size()) {
                Ordering::Less => bv2 = bv2.extract(bv1.get_size() - 1, 0),
                Ordering::Greater => bv2 = bv2.zero_ext(bv1.get_size() - bv2.get_size()),
                _ => {}
            }
            Some(bv1.bvlshr(&bv2))
        }
        PcodeOperation::IntSignedRightShift { .. } => {
            let bv1 = arg(0)?;
            let mut bv2 = arg(1)?.clone();
            match bv1.get_size().cmp(&bv2.get_size()) {
                Ordering::Less => bv2 = bv2.extract(bv1.get_size() - 1, 0),
                Ordering::Greater => bv2 = bv2.zero_ext(bv1.get_size() - bv2.get_size()),
                _ => {}
            }
            Some(bv1.bvashr(&bv2))
        }
        PcodeOperation::IntLeftShift { .. } => {
            let bv1 = arg(0)?;
            let mut bv2 = arg(1)?.clone();
            match bv1.get_size().cmp(&bv2.get_size()) {
                Ordering::Less => bv2 = bv2.extract(bv1.get_size() - 1, 0),
                Ordering::Greater => bv2 = bv2.zero_ext(bv1.get_size() - bv2.get_size()),
                _ => {}
            }
            Some(bv1.bvshl(&bv2))
        }
        PcodeOperation::IntCarry { output: _, .. } => {
            let in0 = arg(0)?;
            let in1 = arg(1)?;
            let carry_bool = in0.bvadd_no_overflow(in1, false);
            let out_bv = carry_bool.ite(&BV::from_i64(0, 8), &BV::from_i64(1, 8));
            // output is typically 1 byte; mirror memory semantics and return 8-bit BV
            Some(out_bv)
        }
        PcodeOperation::IntSignedCarry { output: _, .. } => {
            let in0 = arg(0)?;
            let in1 = arg(1)?;
            let carry_bool = in0.bvadd_no_overflow(in1, true);
            let out_bv = carry_bool.ite(&BV::from_i64(0, 8), &BV::from_i64(1, 8));
            Some(out_bv)
        }
        PcodeOperation::IntSignedBorrow { output: _, .. } => {
            let in0 = arg(0)?;
            let in1 = arg(1)?;
            let borrow_bool = in0.bvsub_no_underflow(in1, true);
            let out_bv = borrow_bool.ite(&BV::from_i64(0, 8), &BV::from_i64(1, 8));
            Some(out_bv)
        }
        PcodeOperation::Int2Comp { .. } => {
            let in0 = arg(0)?;
            let flipped = in0.bvneg().bvadd(BV::from_u64(1, in0.get_size()));
            Some(flipped)
        }
        PcodeOperation::IntSignedLess { output: _, .. } => {
            let in0 = arg(0)?;
            let in1 = arg(1)?;
            let out_bool = in0.bvslt(in1);
            let out_bv = out_bool.ite(&BV::from_i64(1, 8), &BV::from_i64(0, 8));
            Some(out_bv)
        }
        PcodeOperation::IntSignedLessEqual { output: _, .. } => {
            let in0 = arg(0)?;
            let in1 = arg(1)?;
            let out_bool = in0.bvsle(in1);
            let out_bv = out_bool.ite(&BV::from_i64(1, 8), &BV::from_i64(0, 8));
            Some(out_bv)
        }
        PcodeOperation::IntLess { output: _, .. } => {
            let in0 = arg(0)?;
            let in1 = arg(1)?;
            let out_bool = in0.bvult(in1);
            let out_bv = out_bool.ite(&BV::from_i64(1, 8), &BV::from_i64(0, 8));
            Some(out_bv)
        }
        PcodeOperation::IntLessEqual { output: _, .. } => {
            let in0 = arg(0)?;
            let in1 = arg(1)?;
            let out_bool = in0.bvule(in1);
            let out_bv = out_bool.ite(&BV::from_i64(1, 8), &BV::from_i64(0, 8));
            Some(out_bv)
        }
        PcodeOperation::IntEqual { output, .. } => {
            let in0 = arg(0)?;
            let in1 = arg(1)?;
            let outsize = output.size as u32;
            let out_bool = in0.eq(in1);
            let out_bv = out_bool.ite(&BV::from_i64(1, outsize * 8), &BV::from_i64(0, outsize * 8));
            Some(out_bv)
        }
        PcodeOperation::IntNotEqual { output, .. } => {
            let in0 = arg(0)?;
            let in1 = arg(1)?;
            let outsize = output.size as u32;
            let out_bool = in0.eq(in1).not();
            let out_bv = out_bool.ite(&BV::from_i64(1, outsize * 8), &BV::from_i64(0, outsize * 8));
            Some(out_bv)
        }
        PcodeOperation::BoolAnd { .. } => {
            let in0 = arg(0)?;
            let in1 = arg(1)?;
            // mirror memory: compute bitwise and and trim to 1 bit
            let result = in0.bvand(in1).bvand(BV::from_i64(1, 1));
            Some(result)
        }
        PcodeOperation::BoolNegate { .. } => {
            let val = arg(0)?;
            let negated = val.bvneg().bvand(BV::from_i64(1, 1));
            Some(negated)
        }
        PcodeOperation::BoolOr { .. } => {
            let i0 = arg(0)?;
            let i1 = arg(1)?;
            let result = i0.bvor(i1).bvand(BV::from_i64(1, 1));
            Some(result)
        }
        PcodeOperation::BoolXor { .. } => {
            let i0 = arg(0)?;
            let i1 = arg(1)?;
            let result = i0.bvxor(i1).bvand(BV::from_i64(1, 1));
            Some(result)
        }
        PcodeOperation::PopCount { output, .. } => {
            let size = output.size as u32;
            let in0 = arg(0)?;
            let mut outbv = BV::from_i64(0, output.size as u32 * 8);
            for i in 0..size * 8 {
                let extract = in0.extract(i, i);
                let extend = extract.zero_ext((size * 8) - 1);
                outbv = outbv.bvadd(&extend);
            }
            Some(outbv)
        }
        PcodeOperation::SubPiece {
            input0,
            input1,
            output,
        } => {
            let bv0 = arg(0)?;
            // sleigh asserts that input1 is a constant
            let input_low_byte = input1.offset as u32;
            let input_size = (input0.size as u32).saturating_sub(input_low_byte);
            let output_size = output.size as u32;
            let size = min(input_size, output_size);
            let input = bv0.extract((input_low_byte + size) * 8 - 1, input_low_byte * 8);
            let res = match size.cmp(&output_size) {
                Ordering::Less => input.zero_ext((output_size - size) * 8),
                Ordering::Greater => input.extract(output_size * 8 - 1, 0),
                Ordering::Equal => input,
            };
            Some(res)
        }
        PcodeOperation::CallOther { .. } => None,

        PcodeOperation::Call { .. } => None,
        // control-flow and memory-indirect operations don't have a single BV result
        PcodeOperation::Branch { .. }
        | PcodeOperation::CBranch { .. }
        | PcodeOperation::BranchInd { .. }
        | PcodeOperation::CallInd { .. }
        | PcodeOperation::Return { .. } => None,
        // default: unmodeled -> no BV expression
        _ => None,
    }
}
