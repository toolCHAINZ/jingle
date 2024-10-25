use std::cmp::{min, Ordering};
use std::hash::{DefaultHasher, Hash};
use z3::ast::BV;
use jingle_sleigh::{GeneralizedVarNode, PcodeOperation};
use crate::JingleError;
use crate::modeling::{BlockConditionalBranchInfo, State};

pub fn apply_pcode_op(state: &State, op: &PcodeOperation) -> Result<State, JingleError>{
    match op {
        PcodeOperation::Copy { input, output } => {
            let val = self.read_and_track(input.into())?;
            let metadata = self.get_original_state().read_varnode_metadata(input)?;
            self.get_final_state_mut()
                .write_varnode_metadata(output, metadata)?;
            self.write(&output.into(), val)
        }
        PcodeOperation::IntZExt { input, output } => {
            let diff = (output.size - input.size) as u32;
            let val = self.read_and_track(input.into())?;
            let zext = val.zero_ext(diff * 8);
            self.write(&output.into(), zext)
        }
        PcodeOperation::IntSExt { input, output } => {
            let diff = (output.size - input.size) as u32;
            let val = self.read_and_track(input.into())?;
            let zext = val.sign_ext(diff * 8);
            self.write(&output.into(), zext)
        }
        PcodeOperation::Store { output, input } => {
            // read the input we need to STORE
            let bv = self.read_and_track(input.into())?;
            // write the input to the proper space, at the offset we read
            self.write(&output.into(), bv)
        }
        PcodeOperation::Load { input, output } => {
            // read the input we need to LOAD
            let bv = self.read_and_track(input.into())?;
            // read the stored offset for the LOAD destination
            // write the loaded input to the output
            self.write(&output.into(), bv)
        }
        PcodeOperation::IntAdd {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let bv2 = self.read_and_track(input1.into())?;
            let add = bv1 + bv2;
            self.write(&output.into(), add)
        }
        PcodeOperation::IntSub {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let bv2 = self.read_and_track(input1.into())?;
            let add = bv1 - bv2;
            self.write(&output.into(), add)
        }
        PcodeOperation::IntAnd {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let bv2 = self.read_and_track(input1.into())?;
            let and = bv1.bvand(&bv2);
            self.write(&output.into(), and)
        }
        PcodeOperation::IntXor {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let bv2 = self.read_and_track(input1.into())?;
            let and = bv1.bvxor(&bv2);
            self.write(&output.into(), and)
        }
        PcodeOperation::IntOr {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let bv2 = self.read_and_track(input1.into())?;
            let or = bv1.bvor(&bv2);
            self.write(&output.into(), or)
        }
        PcodeOperation::IntNegate { input, output } => {
            let bv = self.read_and_track(input.into())?;
            let neg = bv.neg();
            self.write(&output.into(), neg)
        }
        PcodeOperation::IntMult {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let bv2 = self.read_and_track(input1.into())?;
            let mul = bv1.bvmul(&bv2);
            self.write(&output.into(), mul)
        }
        PcodeOperation::IntDiv {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let bv2 = self.read_and_track(input1.into())?;
            let mul = bv1.bvudiv(&bv2);
            self.write(&output.into(), mul)
        }
        PcodeOperation::IntSignedDiv {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let bv2 = self.read_and_track(input1.into())?;
            let mul = bv1.bvsdiv(&bv2);
            self.write(&output.into(), mul)
        }
        PcodeOperation::IntRem {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let bv2 = self.read_and_track(input1.into())?;
            let mul = bv1.bvurem(&bv2);
            self.write(&output.into(), mul)
        }
        PcodeOperation::IntSignedRem {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let bv2 = self.read_and_track(input1.into())?;
            let mul = bv1.bvsrem(&bv2);
            self.write(&output.into(), mul)
        }
        PcodeOperation::IntRightShift {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let mut bv2 = self.read_and_track(input1.into())?;
            match bv1.get_size().cmp(&bv2.get_size()) {
                Ordering::Less => bv2 = bv2.extract(bv1.get_size() - 1, 0),
                Ordering::Greater => bv2 = bv2.zero_ext(bv1.get_size() - bv2.get_size()),
                _ => {}
            }
            let rshift = bv1.bvlshr(&bv2);
            self.write(&output.into(), rshift)
        }
        PcodeOperation::IntSignedRightShift {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let mut bv2 = self.read_and_track(input1.into())?;
            match bv1.get_size().cmp(&bv2.get_size()) {
                Ordering::Less => bv2 = bv2.extract(bv1.get_size() - 1, 0),
                Ordering::Greater => bv2 = bv2.zero_ext(bv1.get_size() - bv2.get_size()),
                _ => {}
            }
            let rshift = bv1.bvashr(&bv2);
            self.write(&output.into(), rshift)
        }
        PcodeOperation::IntLeftShift {
            input0,
            input1,
            output,
        } => {
            let bv1 = self.read_and_track(input0.into())?;
            let mut bv2 = self.read_and_track(input1.into())?;
            match bv1.get_size().cmp(&bv2.get_size()) {
                Ordering::Less => bv2 = bv2.extract(bv1.get_size() - 1, 0),
                Ordering::Greater => bv2 = bv2.zero_ext(bv1.get_size() - bv2.get_size()),
                _ => {}
            }
            let lshift = bv1.bvshl(&bv2);
            self.write(&output.into(), lshift)
        }
        PcodeOperation::IntCarry {
            input0,
            input1,
            output,
        } => {
            let in0 = self.read_and_track(input0.into())?;
            let in1 = self.read_and_track(input1.into())?;
            // bool arg seems to be for whether this check is signed
            let carry_bool = in0.bvadd_no_overflow(&in1, false);
            let out_bv = carry_bool.ite(
                &BV::from_i64(self.get_z3(), 0, 8),
                &BV::from_i64(self.get_z3(), 1, 8),
            );
            self.write(&output.into(), out_bv)
        }
        PcodeOperation::IntSignedCarry {
            input0,
            input1,
            output,
        } => {
            let in0 = self.read_and_track(input0.into())?;
            let in1 = self.read_and_track(input1.into())?;
            // bool arg seems to be for whether this check is signed
            let carry_bool = in0.bvadd_no_overflow(&in1, true);
            let out_bv = carry_bool.ite(
                &BV::from_i64(self.get_z3(), 0, 8),
                &BV::from_i64(self.get_z3(), 1, 8),
            );
            self.write(&output.into(), out_bv)
        }
        PcodeOperation::IntSignedBorrow {
            input0,
            input1,
            output,
        } => {
            let in0 = self.read_and_track(input0.into())?;
            let in1 = self.read_and_track(input1.into())?;
            // todo: need to do some experimentation as to what the intended
            // meaning of "overflow" is in sleigh vs what it means in z3
            let borrow_bool = in0.bvsub_no_underflow(&in1, true);
            let out_bv = borrow_bool.ite(
                &BV::from_i64(self.get_z3(), 0, 8),
                &BV::from_i64(self.get_z3(), 1, 8),
            );
            self.write(&output.into(), out_bv)
        }
        PcodeOperation::Int2Comp { input, output } => {
            let in0 = self.read_and_track(input.into())?;
            let flipped = in0
                .bvneg()
                .add(BV::from_u64(self.get_z3(), 1, in0.get_size()));
            self.write(&output.into(), flipped)
        }
        PcodeOperation::IntSignedLess {
            input0,
            input1,
            output,
        } => {
            let in0 = self.read_and_track(input0.into())?;
            let in1 = self.read_and_track(input1.into())?;
            let out_bool = in0.bvslt(&in1);
            let out_bv = out_bool.ite(
                &BV::from_i64(self.get_z3(), 1, 8),
                &BV::from_i64(self.get_z3(), 0, 8),
            );
            self.write(&output.into(), out_bv)
        }
        PcodeOperation::IntSignedLessEqual {
            input0,
            input1,
            output,
        } => {
            let in0 = self.read_and_track(input0.into())?;
            let in1 = self.read_and_track(input1.into())?;
            let out_bool = in0.bvsle(&in1);
            let out_bv = out_bool.ite(
                &BV::from_i64(self.get_z3(), 1, 8),
                &BV::from_i64(self.get_z3(), 0, 8),
            );
            self.write(&output.into(), out_bv)
        }
        PcodeOperation::IntLess {
            input0,
            input1,
            output,
        } => {
            let in0 = self.read_and_track(input0.into())?;
            let in1 = self.read_and_track(input1.into())?;
            let out_bool = in0.bvult(&in1);
            let out_bv = out_bool.ite(
                &BV::from_i64(self.get_z3(), 1, 8),
                &BV::from_i64(self.get_z3(), 0, 8),
            );
            self.write(&output.into(), out_bv)
        }
        PcodeOperation::IntLessEqual {
            input0,
            input1,
            output,
        } => {
            let in0 = self.read_and_track(input0.into())?;
            let in1 = self.read_and_track(input1.into())?;
            let out_bool = in0.bvule(&in1);
            let out_bv = out_bool.ite(
                &BV::from_i64(self.get_z3(), 1, 8),
                &BV::from_i64(self.get_z3(), 0, 8),
            );
            self.write(&output.into(), out_bv)
        }
        PcodeOperation::IntEqual {
            input0,
            input1,
            output,
        } => {
            let in0 = self.read_and_track(input0.into())?;
            let in1 = self.read_and_track(input1.into())?;
            let outsize = output.size as u32;
            let out_bool = in0._eq(&in1);
            let out_bv = out_bool.ite(
                &BV::from_i64(self.get_z3(), 1, outsize * 8),
                &BV::from_i64(self.get_z3(), 0, outsize * 8),
            );
            self.write(&output.into(), out_bv)
        }
        PcodeOperation::IntNotEqual {
            input0,
            input1,
            output,
        } => {
            let in0 = self.read_and_track(input0.into())?;
            let in1 = self.read_and_track(input1.into())?;
            let outsize = output.size as u32;
            let out_bool = in0._eq(&in1).not();
            let out_bv = out_bool.ite(
                &BV::from_i64(self.get_z3(), 1, outsize * 8),
                &BV::from_i64(self.get_z3(), 0, outsize * 8),
            );
            self.write(&output.into(), out_bv)
        }
        PcodeOperation::BoolAnd {
            input0,
            input1,
            output,
        } => {
            let i0 = self.read_and_track(input0.into())?;
            let i1 = self.read_and_track(input1.into())?;
            let result = i0
                .bvand(&i1)
                .bvand(&BV::from_u64(self.get_z3(), 1, i0.get_size()));
            self.write(&output.into(), result)
        }
        PcodeOperation::BoolNegate { input, output } => {
            let val = self.read_and_track(input.into())?;
            let negated = val
                .bvneg()
                .bvand(&BV::from_u64(self.get_z3(), 1, val.get_size()));
            self.write(&output.into(), negated)
        }
        PcodeOperation::BoolOr {
            input0,
            input1,
            output,
        } => {
            let i0 = self.read_and_track(input0.into())?;
            let i1 = self.read_and_track(input1.into())?;
            let result = i0
                .bvor(&i1)
                .bvand(&BV::from_u64(self.get_z3(), 1, i0.get_size()));
            self.write(&output.into(), result)
        }
        PcodeOperation::BoolXor {
            input0,
            input1,
            output,
        } => {
            let i0 = self.read_and_track(input0.into())?;
            let i1 = self.read_and_track(input1.into())?;
            let result = i0
                .bvxor(&i1)
                .bvand(&BV::from_u64(self.get_z3(), 1, i0.get_size()));
            self.write(&output.into(), result)
        }
        PcodeOperation::PopCount { input, output } => {
            let size = output.size as u32;
            let in0 = self.read_and_track(input.into())?;
            let mut outbv = BV::from_i64(self.get_z3(), 0, output.size as u32 * 8);
            for i in 0..size * 8 {
                let extract = in0.extract(i, i);
                let extend = extract.zero_ext((size * 8) - 1);
                outbv = outbv.bvadd(&extend);
            }

            self.write(&output.into(), outbv)
        }
        PcodeOperation::Branch { input } => {
            self.get_branch_builder()
                .set_last(&GeneralizedVarNode::from(input));
            self.read_and_track(GeneralizedVarNode::from(input))?;
            Ok(())
        }
        PcodeOperation::BranchInd { input } => {
            self.get_branch_builder()
                .set_last(&GeneralizedVarNode::from(input));
            self.read_and_track(GeneralizedVarNode::from(&input.pointer_location))?;
            Ok(())
        }
        PcodeOperation::Call { input } => {
            self.get_branch_builder().set_last(&input.into());
            self.read_and_track(input.into())?;
            Ok(())
        }
        PcodeOperation::CBranch { input0, input1 } => {
            self.get_branch_builder()
                .push_conditional(&BlockConditionalBranchInfo {
                    condition: input1.clone(),
                    destination: input0.into(),
                });
            self.read_and_track(input0.into())?;
            self.read_and_track(input1.into())?;
            Ok(())
        }
        PcodeOperation::SubPiece {
            input0,
            input1,
            output,
        } => {
            let bv0 = self.read_and_track(input0.into())?;
            // sleigh asserts that input1 is a constant
            let input_low_byte = input1.offset as u32;
            let input_size = (input0.size as u32) - input_low_byte;
            let output_size = output.size as u32;
            let size = min(input_size, output_size);
            let input = bv0.extract((input_low_byte + size) * 8 - 1, input_low_byte * 8);
            match size.cmp(&output_size) {
                Ordering::Less => {
                    self.write(&output.into(), input.zero_ext((output_size - size) * 8))
                }
                Ordering::Greater => {
                    self.write(&output.into(), input.extract(output_size * 8 - 1, 0))
                }
                Ordering::Equal => self.write(&output.into(), input),
            }
        }
        PcodeOperation::CallOther { inputs, output } => {
            let mut hasher = DefaultHasher::new();
            for vn in inputs {
                vn.hash(&mut hasher);
            }
            let hash = hasher.finish();
            for input in inputs.iter() {
                self.read_and_track(input.into())?;
            }
            let hash_vn = self.get_final_state().varnode(
                "const",
                hash,
                self.get_final_state()
                    .get_default_code_space_info()
                    .index_size_bytes as usize,
            )?;
            let metadata = self
                .get_final_state()
                .immediate_metadata_array(true, hash_vn.size);
            self.get_final_state_mut()
                .write_varnode_metadata(&hash_vn, metadata)?;
            self.get_branch_builder().set_last(&hash_vn.into());
            if let Some(out) = output {
                let size = out.size * 8;
                let hash_bv = BV::from_u64(self.get_z3(), hash, size as u32);
                let metadata = self
                    .get_final_state()
                    .immediate_metadata_array(true, out.size);
                self.get_final_state_mut()
                    .write_varnode_metadata(out, metadata)?;
                self.write(&out.into(), hash_bv)?;
            }
            Ok(())
        }
        PcodeOperation::CallInd { input } => {
            self.get_branch_builder()
                .set_last(&GeneralizedVarNode::from(input));
            self.read_and_track(GeneralizedVarNode::from(&input.pointer_location))?;
            Ok(())
        }
        PcodeOperation::Return { input } => {
            self.get_branch_builder()
                .set_last(&GeneralizedVarNode::from(input));
            self.read_and_track(GeneralizedVarNode::from(&input.pointer_location))?;
            Ok(())
        }
        v => Err(JingleError::UnmodeledInstruction(Box::new(v.clone()))),
    }
}