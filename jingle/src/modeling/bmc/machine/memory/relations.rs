use jingle_sleigh::ArchInfoProvider;
use crate::modeling::bmc::machine::memory::space::BMCModeledSpace;
use crate::modeling::bmc::machine::memory::MemoryState;
use crate::JingleError;
use jingle_sleigh::{PcodeOperation, SpaceManager, SpaceType, VarNode};
use std::cmp::{min, Ordering};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::{Add, Neg};
use z3::ast::{Ast, BV};

impl<'ctx> MemoryState<'ctx> {
    pub fn apply(&self, op: &PcodeOperation) -> Result<Self, JingleError> {
        let mut final_state = self.clone();
        match &op {
            PcodeOperation::Copy { input, output } => {
                let val = self.read(input)?;
                final_state.write(output, val)
            }
            PcodeOperation::IntZExt { input, output } => {
                let diff = (output.size - input.size) as u32;
                let val = self.read(input)?;
                let zext = val.zero_ext(diff * 8);
                final_state.write(output, zext)
            }
            PcodeOperation::IntSExt { input, output } => {
                let diff = (output.size - input.size) as u32;
                let val = self.read(input)?;
                let zext = val.sign_ext(diff * 8);
                final_state.write(output, zext)
            }
            PcodeOperation::Store { output, input } => {
                // read the input we need to STORE
                let bv = self.read(input)?;
                // write the input to the proper space, at the offset we read
                final_state.write(output, bv)
            }
            PcodeOperation::Load { input, output } => {
                // read the input we need to LOAD
                let bv = self.read(input)?;
                // read the stored offset for the LOAD destination
                // write the loaded input to the output
                final_state.write(output, bv)
            }
            PcodeOperation::IntAdd {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let bv2 = self.read(input1)?;
                let add = bv1 + bv2;
                final_state.write(output, add)
            }
            PcodeOperation::IntSub {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let bv2 = self.read(input1)?;
                let sub = bv1 - bv2;
                final_state.write(output, sub)
            }
            PcodeOperation::IntAnd {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let bv2 = self.read(input1)?;
                let and = bv1.bvand(&bv2);
                final_state.write(output, and)
            }
            PcodeOperation::IntXor {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let bv2 = self.read(input1)?;
                let and = bv1.bvxor(&bv2);
                final_state.write(output, and)
            }
            PcodeOperation::IntOr {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let bv2 = self.read(input1)?;
                let or = bv1.bvor(&bv2);
                final_state.write(output, or)
            }
            PcodeOperation::IntNegate { input, output } => {
                let bv = self.read(input)?;
                let neg = bv.neg();
                final_state.write(output, neg)
            }
            PcodeOperation::IntMult {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let bv2 = self.read(input1)?;
                let mul = bv1.bvmul(&bv2);
                final_state.write(output, mul)
            }
            PcodeOperation::IntDiv {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let bv2 = self.read(input1)?;
                let mul = bv1.bvudiv(&bv2);
                final_state.write(output, mul)
            }
            PcodeOperation::IntSignedDiv {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let bv2 = self.read(input1)?;
                let mul = bv1.bvsdiv(&bv2);
                final_state.write(output, mul)
            }
            PcodeOperation::IntRem {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let bv2 = self.read(input1)?;
                let mul = bv1.bvurem(&bv2);
                final_state.write(output, mul)
            }
            PcodeOperation::IntSignedRem {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let bv2 = self.read(input1)?;
                let mul = bv1.bvsrem(&bv2);
                final_state.write(output, mul)
            }
            PcodeOperation::IntRightShift {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let mut bv2 = self.read(input1)?;
                match bv1.get_size().cmp(&bv2.get_size()) {
                    Ordering::Less => bv2 = bv2.extract(bv1.get_size() - 1, 0),
                    Ordering::Greater => bv2 = bv2.zero_ext(bv1.get_size() - bv2.get_size()),
                    _ => {}
                }
                let rshift = bv1.bvlshr(&bv2);
                final_state.write(output, rshift)
            }
            PcodeOperation::IntSignedRightShift {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let mut bv2 = self.read(input1)?;
                match bv1.get_size().cmp(&bv2.get_size()) {
                    Ordering::Less => bv2 = bv2.extract(bv1.get_size() - 1, 0),
                    Ordering::Greater => bv2 = bv2.zero_ext(bv1.get_size() - bv2.get_size()),
                    _ => {}
                }
                let rshift = bv1.bvashr(&bv2);
                final_state.write(output, rshift)
            }
            PcodeOperation::IntLeftShift {
                input0,
                input1,
                output,
            } => {
                let bv1 = self.read(input0)?;
                let mut bv2 = self.read(input1)?;
                match bv1.get_size().cmp(&bv2.get_size()) {
                    Ordering::Less => bv2 = bv2.extract(bv1.get_size() - 1, 0),
                    Ordering::Greater => bv2 = bv2.zero_ext(bv1.get_size() - bv2.get_size()),
                    _ => {}
                }
                let lshift = bv1.bvshl(&bv2);
                final_state.write(output, lshift)
            }
            PcodeOperation::IntCarry {
                input0,
                input1,
                output,
            } => {
                let in0 = self.read(input0)?;
                let in1 = self.read(input1)?;
                // bool arg seems to be for whether this check is signed
                let carry_bool = in0.bvadd_no_overflow(&in1, false);
                let out_bv = carry_bool.ite(
                    &BV::from_i64(self.jingle.z3, 0, 8),
                    &BV::from_i64(self.jingle.z3, 1, 8),
                );
                final_state.write(output, out_bv)
            }
            PcodeOperation::IntSignedCarry {
                input0,
                input1,
                output,
            } => {
                let in0 = self.read(input0)?;
                let in1 = self.read(input1)?;
                // bool arg seems to be for whether this check is signed
                let carry_bool = in0.bvadd_no_overflow(&in1, true);
                let out_bv = carry_bool.ite(
                    &BV::from_i64(self.jingle.z3, 0, 8),
                    &BV::from_i64(self.jingle.z3, 1, 8),
                );
                final_state.write(output, out_bv)
            }
            PcodeOperation::IntSignedBorrow {
                input0,
                input1,
                output,
            } => {
                let in0 = self.read(input0)?;
                let in1 = self.read(input1)?;
                // bool arg seems to be for whether this check is signed
                // todo: need to do some experimentation as to what the intended
                // meaning of "overflow" is in sleigh vs what it means in z3
                let borrow_bool = in0.bvsub_no_underflow(&in1, true);
                let out_bv = borrow_bool.ite(
                    &BV::from_i64(self.jingle.z3, 0, 8),
                    &BV::from_i64(self.jingle.z3, 1, 8),
                );
                final_state.write(output, out_bv)
            }
            PcodeOperation::Int2Comp { input, output } => {
                let in0 = self.read(input)?;
                let flipped = in0
                    .bvneg()
                    .add(BV::from_u64(self.jingle.z3, 1, in0.get_size()));
                final_state.write(output, flipped)
            }
            PcodeOperation::IntSignedLess {
                input0,
                input1,
                output,
            } => {
                let in0 = self.read(input0)?;
                let in1 = self.read(input1)?;
                // bool arg seems to be for whether this check is signed
                let out_bool = in0.bvslt(&in1);
                let out_bv = out_bool.ite(
                    &BV::from_i64(self.jingle.z3, 1, 8),
                    &BV::from_i64(self.jingle.z3, 0, 8),
                );
                final_state.write(output, out_bv)
            }
            PcodeOperation::IntSignedLessEqual {
                input0,
                input1,
                output,
            } => {
                let in0 = self.read(input0)?;
                let in1 = self.read(input1)?;
                // bool arg seems to be for whether this check is signed
                let out_bool = in0.bvsle(&in1);
                let out_bv = out_bool.ite(
                    &BV::from_i64(self.jingle.z3, 1, 8),
                    &BV::from_i64(self.jingle.z3, 0, 8),
                );
                final_state.write(output, out_bv)
            }
            PcodeOperation::IntLess {
                input0,
                input1,
                output,
            } => {
                let in0 = self.read(input0)?;
                let in1 = self.read(input1)?;
                // bool arg seems to be for whether this check is signed
                let out_bool = in0.bvult(&in1);
                let out_bv = out_bool.ite(
                    &BV::from_i64(self.jingle.z3, 1, 8),
                    &BV::from_i64(self.jingle.z3, 0, 8),
                );
                final_state.write(output, out_bv)
            }
            PcodeOperation::IntLessEqual {
                input0,
                input1,
                output,
            } => {
                let in0 = self.read(input0)?;
                let in1 = self.read(input1)?;
                // bool arg seems to be for whether this check is signed
                let out_bool = in0.bvule(&in1);
                let out_bv = out_bool.ite(
                    &BV::from_i64(self.jingle.z3, 1, 8),
                    &BV::from_i64(self.jingle.z3, 0, 8),
                );
                final_state.write(output, out_bv)
            }
            PcodeOperation::IntEqual {
                input0,
                input1,
                output,
            } => {
                let in0 = self.read(input0)?;
                let in1 = self.read(input1)?;
                // bool arg seems to be for whether this check is signed
                let outsize = output.size as u32;
                let out_bool = in0._eq(&in1);
                let out_bv = out_bool.ite(
                    &BV::from_i64(self.jingle.z3, 1, outsize * 8),
                    &BV::from_i64(self.jingle.z3, 0, outsize * 8),
                );
                final_state.write(output, out_bv)
            }
            PcodeOperation::IntNotEqual {
                input0,
                input1,
                output,
            } => {
                let in0 = self.read(input0)?;
                let in1 = self.read(input1)?;
                // bool arg seems to be for whether this check is signed
                let outsize = output.size as u32;
                let out_bool = in0._eq(&in1).not();
                let out_bv = out_bool.ite(
                    &BV::from_i64(self.jingle.z3, 1, outsize * 8),
                    &BV::from_i64(self.jingle.z3, 0, outsize * 8),
                );
                final_state.write(output, out_bv)
            }
            PcodeOperation::BoolAnd {
                input0,
                input1,
                output,
            } => {
                let in0 = self.read(input0)?;
                let in1 = self.read(input1)?;
                // bool arg seems to be for whether this check is signed
                let result =
                    in0.bvand(&in1)
                        .bvand(&BV::from_u64(self.jingle.z3, 1, in0.get_size()));
                final_state.write(output, result)
            }
            PcodeOperation::BoolNegate { input, output } => {
                let val = self.read(input)?;
                let negated = val
                    .bvneg()
                    .bvand(&BV::from_u64(self.jingle.z3, 1, val.get_size()));
                final_state.write(output, negated)
            }
            PcodeOperation::BoolOr {
                input0,
                input1,
                output,
            } => {
                let i0 = self.read(input0)?;
                let i1 = self.read(input1)?;
                // bool arg seems to be for whether this check is signed
                let result = i0
                    .bvor(&i1)
                    .bvand(&BV::from_u64(self.jingle.z3, 1, i0.get_size()));
                final_state.write(output, result)
            }
            PcodeOperation::BoolXor {
                input0,
                input1,
                output,
            } => {
                let i0 = self.read(input0)?;
                let i1 = self.read(input1)?;
                // bool arg seems to be for whether this check is signed
                let result = i0
                    .bvxor(&i1)
                    .bvand(&BV::from_u64(self.jingle.z3, 1, i0.get_size()));
                final_state.write(output, result)
            }
            PcodeOperation::PopCount { input, output } => {
                let size = output.size as u32;
                let in0 = self.read(input)?;
                let mut outbv = BV::from_i64(self.jingle.z3, 0, output.size as u32 * 8);
                for i in 0..size * 8 {
                    let extract = in0.extract(i, i);
                    let extend = extract.zero_ext((size * 8) - 1);
                    outbv = outbv.bvadd(&extend);
                }

                final_state.write(output, outbv)
            }
            PcodeOperation::SubPiece {
                input0,
                input1,
                output,
            } => {
                let bv0 = self.read(input0)?;
                // sleigh asserts that input1 is a constant
                let input_low_byte = input1.offset as u32;
                let input_size = (input0.size as u32) - input_low_byte;
                let output_size = output.size as u32;
                let size = min(input_size, output_size);
                let input = bv0.extract((input_low_byte + size) * 8 - 1, input_low_byte * 8);
                match size.cmp(&output_size) {
                    Ordering::Less => {
                        final_state.write(output, input.zero_ext((output_size - size) * 8))
                    }
                    Ordering::Greater => {
                        final_state.write(output, input.extract(output_size * 8 - 1, 0))
                    }
                    Ordering::Equal => final_state.write(output, input),
                }
            }
            PcodeOperation::CallOther { inputs, output } => {
                let mut hasher = DefaultHasher::new();
                for vn in inputs {
                    vn.hash(&mut hasher);
                }
                let hash = hasher.finish();
                if let Some(out) = output {
                    let size = out.size * 8;
                    let hash_bv = BV::from_u64(self.jingle.z3, hash, size as u32);
                    final_state.write(out, hash_bv)
                } else {
                    Ok(final_state)
                }
            }
            PcodeOperation::Branch { input } => {
                final_state.conditional_clear_internal_space(input);
                Ok(final_state)
            }
            PcodeOperation::CBranch { input0, .. } => {
                final_state.conditional_clear_internal_space(input0);
                Ok(final_state)
            }
            PcodeOperation::BranchInd { .. }
            | PcodeOperation::Call { .. }
            | PcodeOperation::CallInd { .. }
            | PcodeOperation::Return { .. } => {
                final_state.clear_internal_space();
                Ok(final_state)
            }
            v => Err(JingleError::UnmodeledInstruction(Box::new((*v).clone()))),
        }
    }
}
