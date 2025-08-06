use crate::error::JingleError;

use crate::varnode::ResolvedVarnode::{Direct, Indirect};
use crate::varnode::{ResolvedIndirectVarNode, ResolvedVarnode};
use jingle_sleigh::{ArchInfoProvider, GeneralizedVarNode, PcodeOperation, SpaceType};
use std::cmp::{Ordering, min};
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::{Add, Neg};
use tracing::instrument;
use z3::ast::{Ast, BV, Bool};

mod block;
mod branch;
mod concretize;
mod instruction;
pub mod machine;
mod slice;
mod state;
pub mod tactics;

use crate::JingleContext;
pub use block::ModeledBlock;
pub use branch::*;
pub use instruction::ModeledInstruction;
pub use state::State;

/// `jingle` models straight-line traces of computations. This trait represents all the information
/// needed to model a given trace.
/// It enforces that the type has a handle to z3, has a concept of program state, and also
/// defines several helper functions for building formulae
/// todo: this should probably be separated out with the extension trait pattern
pub trait ModelingContext: ArchInfoProvider + Debug + Sized {
    /// Get a handle to the jingle context associated with this modeling context
    fn get_jingle(&self) -> &JingleContext;

    /// Get the address this context is associated with (e.g. for an instruction, it is the address,
    /// for a basic block, it is the address of the first instruction).
    /// Used for building assertions about branch reachability
    fn get_address(&self) -> u64;

    /// Get the [State] associated with the precondition of this trace
    fn get_original_state(&self) -> &State;
    /// Get the [State] associated with the postcondition of this trace
    fn get_final_state(&self) -> &State;

    /// Get a vec of the operations associated with this trace
    /// todo: should this be a slice instead of a vec?
    /// todo: someday when we support paths this should be a graph and not a vec
    fn get_ops(&self) -> Vec<&PcodeOperation>;
    /// Get a hashset of the addresses read by this trace. The values returned in this hashset are
    /// fully modeled: a read from a given varnode will evaluate to its value at the stage in the
    /// computation that the read was performed. Because of this, these should always be read
    /// from the [State] returned by [get_final_state], as it is guaranteed to have a handle to
    /// all intermediate spaces that may be referenced
    fn get_inputs(&self) -> HashSet<ResolvedVarnode>;
    /// Get a hashset of the addresses written by this trace. The values returned in this hashset
    /// are fully modeled: a read from a given varnode will evaluate to its value at the stage in
    /// the computation that the read was performed. Because of this, these should always be read
    /// from the [State] returned by [get_final_state], as it is guaranteed to have a handle to
    /// all intermediate spaces that may be referenced
    fn get_outputs(&self) -> HashSet<ResolvedVarnode>;

    ///`jingle` supports some rudimentary modeling of control flow; this will return a bitvector
    /// encapsulating the possible end-of-block behaviors of this trace
    fn get_branch_constraint(&self) -> &BranchConstraint;

    /// SLEIGH models instructions using many address spaces, some of which do not map directly to
    /// architectural spaces. For instance, the `unique` space is used as an intra-instruction
    /// "scratch pad" for intermediate results and is explicitly cleared between each instruction.
    /// Therefore, it is often useful to filter a varnode by whether it references an architectural
    /// space, since we do not want to constrain spaces like `unique`.
    fn should_varnode_constrain(&self, v: &ResolvedVarnode) -> bool {
        match v {
            Direct(d) => self
                .get_final_state()
                .get_space_info(d.space_index)
                .map(|o| o._type == SpaceType::IPTR_PROCESSOR)
                .unwrap_or(false),
            Indirect(_) => true,
        }
    }

    /// Returns a [Bool] assertion that [self] upholds the postconditions of [other]
    /// This is done by iterating over all fully-modeled constraining outputs of [other] and
    /// enforcing that the same locations in [self] are equal.
    /// In our procedure, this is only ever called on contexts that we have already verified write
    /// to all outputs that [other] did, eliminating the risk of spurious false positives
    fn upholds_postcondition<T: ModelingContext>(
        &self,
        other: &T,
    ) -> Result<Bool, JingleError> {
        let mut output_terms = vec![];
        for vn in other
            .get_outputs()
            .iter()
            .filter(|v| self.should_varnode_constrain(v))
        {
            let ours = self.get_final_state().read_resolved(vn)?;
            let other_bv = other.get_final_state().read_resolved(vn)?;
            output_terms.push(ours._eq(&other_bv).simplify());
            if let Indirect(a) = vn {
                let ours = self.get_final_state().read_varnode(&a.pointer_location)?;
                let other = other.get_final_state().read_varnode(&a.pointer_location)?;
                output_terms.push(ours._eq(&other).simplify());
            }
        }
        let imp_terms: Vec<&Bool> = output_terms.iter().collect();
        let outputs_pairwise_equal = Bool::and(self.get_jingle().ctx(), imp_terms.as_slice());
        Ok(outputs_pairwise_equal)
    }

    /// Returns an assertion that the final state of [self] and the first state of [other] are
    /// equal. This allows for concatenating two traces into one for the purposes of modeling.
    fn assert_concat<T: ModelingContext>(
        &self,
        other: &T,
    ) -> Result<Bool, JingleError> {
        self.get_final_state()._eq(other.get_original_state())
    }

    /// Returns an assertion that [other]'s end-branch behavior is able to branch to the same
    /// destination as [self], given that [self] has branching behavior
    /// todo: should swap self and other to make this align better with [upholds_postcondition]
    #[deprecated]
    #[expect(deprecated)]
    fn branch_comparison<T: ModelingContext>(
        &self,
        other: &T,
    ) -> Result<Option<Bool>, JingleError> {
        if !self.get_branch_constraint().has_branch() {
            Ok(None)
        } else {
            if !self.get_branch_constraint().conditional_branches.is_empty()
                || !other
                    .get_branch_constraint()
                    .conditional_branches
                    .is_empty()
            {
                return Ok(Some(Bool::from_bool(self.get_jingle().ctx(), false)));
            }
            let self_bv = self.get_branch_constraint().build_bv(self)?;
            let other_bv = other.get_branch_constraint().build_bv(other)?;
            let self_bv = zext_to_match(self_bv, &other_bv);
            let other_bv = zext_to_match(other_bv, &self_bv);
            let self_bv_metadata = self.get_branch_constraint().build_bv_metadata(self)?;
            let other_bv_metadata = other.get_branch_constraint().build_bv_metadata(other)?;
            let self_bv_metadata =
                zext_to_match(self_bv_metadata.simplify(), &other_bv_metadata.simplify());
            let other_bv_metadata = zext_to_match(other_bv_metadata, &self_bv_metadata);
            Ok(Some(Bool::and(
                self.get_jingle().ctx(),
                &[
                    self_bv._eq(&other_bv).simplify(),
                    self_bv_metadata._eq(&other_bv_metadata).simplify(),
                ],
            )))
        }
    }

    /// Returns a [Bool] assertion that the given trace's end-branch behavior is able to
    /// branch to the given [u64]
    #[expect(deprecated)]
    fn can_branch_to_address(&self, addr: u64) -> Result<Bool, JingleError> {
        let branch_constraint = self.get_branch_constraint().build_bv(self)?;
        let addr_bv = BV::from_i64(
            self.get_jingle().ctx(),
            addr as i64,
            branch_constraint.get_size(),
        );
        Ok(branch_constraint._eq(&addr_bv))
    }
}

/// This trait is used for types that build modeling contexts. This could maybe be a single
/// struct instead of a trait.
/// The helper methods in here allow for parsing pcode operations into z3 formulae, and
/// automatically tracking the inputs/outputs of each operation and traces composed thereof
pub(crate) trait TranslationContext: ModelingContext {
    /// Adds a [GeneralizedVarNode] to the "input care set" for this operation.
    /// This is usually used for asserting equality of all input varnodes when
    /// comparing operations
    fn track_input(&mut self, input: &ResolvedVarnode);

    /// Adds a [GeneralizedVarNode] to the "input care set" for this operation.
    /// This is usually used for asserting post-equality and pre-inequality
    /// of all output [GeneralizedVarNode]s when comparing operations
    fn track_output(&mut self, output: &ResolvedVarnode);

    /// Get a mutable handle to the "lastest" state
    fn get_final_state_mut(&mut self) -> &mut State;

    /// Get the helper object for encapsulating branch behavior
    fn get_branch_builder(&mut self) -> &mut BranchConstraint;

    /// A helper function to both read and track an input [VarNode].
    fn read_and_track(
        &mut self,
        gen_varnode: GeneralizedVarNode,
    ) -> Result<BV, JingleError> {
        match gen_varnode {
            GeneralizedVarNode::Direct(d) => {
                self.track_input(&Direct(d.clone()));
                self.get_final_state().read_varnode(&d)
            }
            GeneralizedVarNode::Indirect(indirect) => {
                self.track_input(&Direct(indirect.pointer_location.clone()));
                let pointer = self
                    .get_final_state()
                    .read_varnode(&indirect.pointer_location)?
                    .clone();
                self.track_input(&Indirect(ResolvedIndirectVarNode {
                    pointer,
                    pointer_location: indirect.pointer_location.clone(),
                    access_size_bytes: indirect.access_size_bytes,
                    pointer_space_idx: indirect.pointer_space_index,
                }));
                self.get_final_state().read_varnode_indirect(&indirect)
            }
        }
    }

    fn write(
        &mut self,
        r#gen: &GeneralizedVarNode,
        val: BV,
    ) -> Result<(), JingleError> {
        match r#gen {
            GeneralizedVarNode::Direct(d) => {
                self.track_output(&Direct(d.clone()));
                self.get_final_state_mut().write_varnode(d, val)?;
            }
            GeneralizedVarNode::Indirect(indirect) => {
                let pointer = self.read_and_track(indirect.pointer_location.clone().into())?;
                self.track_output(&Indirect(ResolvedIndirectVarNode {
                    pointer,
                    pointer_location: indirect.pointer_location.clone(),
                    access_size_bytes: indirect.access_size_bytes,
                    pointer_space_idx: indirect.pointer_space_index,
                }));
                self.get_final_state_mut()
                    .write_varnode_indirect(indirect, val)?;
            }
        }
        Ok(())
    }

    /// Apply the updates of a [PcodeOperation] on top of this context.
    #[instrument(skip_all)]
    fn model_pcode_op(&mut self, op: &PcodeOperation) -> Result<(), JingleError>
    where
        Self: Sized,
    {
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
                let mut bv1 = self.read_and_track(input0.into())?;
                let mut bv2 = self.read_and_track(input1.into())?;
                match bv1.get_size().cmp(&bv2.get_size()) {
                    Ordering::Less => bv1 = bv1.zero_ext(bv2.get_size() - bv1.get_size()),
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
                    &BV::from_i64(self.get_jingle().ctx(), 0, 8),
                    &BV::from_i64(self.get_jingle().ctx(), 1, 8),
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
                    &BV::from_i64(self.get_jingle().ctx(), 0, 8),
                    &BV::from_i64(self.get_jingle().ctx(), 1, 8),
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
                    &BV::from_i64(self.get_jingle().ctx(), 0, 8),
                    &BV::from_i64(self.get_jingle().ctx(), 1, 8),
                );
                self.write(&output.into(), out_bv)
            }
            PcodeOperation::Int2Comp { input, output } => {
                let in0 = self.read_and_track(input.into())?;
                let flipped =
                    in0.bvneg()
                        .add(BV::from_u64(self.get_jingle().ctx(), 1, in0.get_size()));
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
                    &BV::from_i64(self.get_jingle().ctx(), 1, 8),
                    &BV::from_i64(self.get_jingle().ctx(), 0, 8),
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
                    &BV::from_i64(self.get_jingle().ctx(), 1, 8),
                    &BV::from_i64(self.get_jingle().ctx(), 0, 8),
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
                    &BV::from_i64(self.get_jingle().ctx(), 1, 8),
                    &BV::from_i64(self.get_jingle().ctx(), 0, 8),
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
                    &BV::from_i64(self.get_jingle().ctx(), 1, 8),
                    &BV::from_i64(self.get_jingle().ctx(), 0, 8),
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
                    &BV::from_i64(self.get_jingle().ctx(), 1, outsize * 8),
                    &BV::from_i64(self.get_jingle().ctx(), 0, outsize * 8),
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
                    &BV::from_i64(self.get_jingle().ctx(), 1, outsize * 8),
                    &BV::from_i64(self.get_jingle().ctx(), 0, outsize * 8),
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
                let result =
                    i0.bvand(&i1)
                        .bvand(&BV::from_u64(self.get_jingle().ctx(), 1, i0.get_size()));
                self.write(&output.into(), result)
            }
            PcodeOperation::BoolNegate { input, output } => {
                let val = self.read_and_track(input.into())?;
                let negated =
                    val.bvneg()
                        .bvand(&BV::from_u64(self.get_jingle().ctx(), 1, val.get_size()));
                self.write(&output.into(), negated)
            }
            PcodeOperation::BoolOr {
                input0,
                input1,
                output,
            } => {
                let i0 = self.read_and_track(input0.into())?;
                let i1 = self.read_and_track(input1.into())?;
                let result =
                    i0.bvor(&i1)
                        .bvand(&BV::from_u64(self.get_jingle().ctx(), 1, i0.get_size()));
                self.write(&output.into(), result)
            }
            PcodeOperation::BoolXor {
                input0,
                input1,
                output,
            } => {
                let i0 = self.read_and_track(input0.into())?;
                let i1 = self.read_and_track(input1.into())?;
                let result =
                    i0.bvxor(&i1)
                        .bvand(&BV::from_u64(self.get_jingle().ctx(), 1, i0.get_size()));
                self.write(&output.into(), result)
            }
            PcodeOperation::PopCount { input, output } => {
                let size = output.size as u32;
                let in0 = self.read_and_track(input.into())?;
                let mut outbv = BV::from_i64(self.get_jingle().ctx(), 0, output.size as u32 * 8);
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
                    let hash_bv = BV::from_u64(self.get_jingle().ctx(), hash, size as u32);
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
}

fn zext_to_match(bv1: BV, bv2: &BV) -> BV {
    if bv1.get_size() < bv2.get_size() {
        bv1.zero_ext(bv2.get_size() - bv1.get_size())
    } else {
        bv1
    }
}
