pub mod branch;

use crate::pcode::PcodeOperation::{
    BoolAnd, BoolNegate, BoolOr, BoolXor, Branch, BranchInd, CBranch, CPoolRef, Call, CallInd,
    CallOther, Cast, Copy, Extract, FloatAbs, FloatAdd, FloatCeil, FloatDiv, FloatEqual,
    FloatFloatToFloat, FloatFloor, FloatIntToFloat, FloatLess, FloatLessEqual, FloatMult, FloatNaN,
    FloatNeg, FloatNotEqual, FloatRound, FloatSqrt, FloatSub, FloatTrunc, Indirect, Insert,
    Int2Comp, IntAdd, IntAnd, IntCarry, IntDiv, IntEqual, IntLeftShift, IntLess, IntLessEqual,
    IntMult, IntNegate, IntNotEqual, IntOr, IntRem, IntRightShift, IntSExt, IntSignedBorrow,
    IntSignedCarry, IntSignedDiv, IntSignedLess, IntSignedLessEqual, IntSignedRem,
    IntSignedRightShift, IntSub, IntXor, IntZExt, Load, LzCount, MultiEqual, New, Piece, PopCount,
    PtrAdd, PtrSub, Return, SegmentOp, Store, SubPiece,
};

use crate::GeneralizedVarNode;
use crate::ffi::instruction::bridge::RawPcodeOp;
pub use crate::ffi::opcode::OpCode;
use crate::varnode::{IndirectVarNode, VarNode};
#[cfg(feature = "pyo3")]
use pyo3::pyclass;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter, LowerHex};

#[cfg_attr(feature = "pyo3", pyclass)]
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PcodeOperation {
    Copy {
        input: VarNode,
        output: VarNode,
    },
    Load {
        input: IndirectVarNode,
        output: VarNode,
    },
    Store {
        output: IndirectVarNode,
        input: VarNode,
    },
    Branch {
        input: VarNode,
    },
    CBranch {
        /// The Branch Destination
        input0: VarNode,
        /// A Boolean [VarNode] indicating whether to take the branch
        input1: VarNode,
    },
    BranchInd {
        input: IndirectVarNode,
    },
    Call {
        input: VarNode,
    },
    /// We're only dealing with raw pcode so this can only have one input
    CallInd {
        input: IndirectVarNode,
    },
    CallOther {
        output: Option<VarNode>,
        inputs: Vec<VarNode>,
    },
    Return {
        input: IndirectVarNode,
    },
    IntEqual {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntNotEqual {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntSignedLess {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntSignedLessEqual {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntLess {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntLessEqual {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntSExt {
        input: VarNode,
        output: VarNode,
    },
    IntZExt {
        input: VarNode,
        output: VarNode,
    },
    IntAdd {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntSub {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntCarry {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntSignedCarry {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntSignedBorrow {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    Int2Comp {
        output: VarNode,
        input: VarNode,
    },
    IntNegate {
        output: VarNode,
        input: VarNode,
    },
    IntXor {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntAnd {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntOr {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntLeftShift {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntRightShift {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntSignedRightShift {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntMult {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntDiv {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntSignedDiv {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntRem {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    IntSignedRem {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    BoolNegate {
        output: VarNode,
        input: VarNode,
    },
    BoolXor {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    BoolAnd {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    BoolOr {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    FloatEqual {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    FloatNotEqual {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    FloatLess {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    FloatLessEqual {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    FloatNaN {
        output: VarNode,
        input: VarNode,
    },
    FloatAdd {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    FloatDiv {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    FloatMult {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    FloatSub {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    FloatNeg {
        output: VarNode,
        input: VarNode,
    },
    FloatAbs {
        output: VarNode,
        input: VarNode,
    },
    FloatSqrt {
        output: VarNode,
        input: VarNode,
    },
    FloatIntToFloat {
        output: VarNode,
        input: VarNode,
    },
    FloatFloatToFloat {
        output: VarNode,
        input: VarNode,
    },
    FloatTrunc {
        output: VarNode,
        input: VarNode,
    },
    FloatCeil {
        output: VarNode,
        input: VarNode,
    },
    FloatFloor {
        output: VarNode,
        input: VarNode,
    },
    FloatRound {
        output: VarNode,
        input: VarNode,
    },
    MultiEqual {
        input0: VarNode,
        input1: VarNode,
        inputs: Vec<VarNode>,
        output: VarNode,
    },
    Indirect {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    Piece {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    SubPiece {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    Cast {
        output: VarNode,
        input: VarNode,
    },
    PtrAdd {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
        /// Must be a constant
        input2: VarNode,
    },
    PtrSub {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
    },
    /// This opcode is undocumented; recovered the shape of it from the ghidra
    /// source, but have not put any effort into determining how it works
    SegmentOp {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
        input2: VarNode,
    },
    CPoolRef {
        input0: VarNode,
        input1: VarNode,
        inputs: Vec<VarNode>,
        output: VarNode,
    },
    New {
        output: VarNode,
        input: VarNode,
        size: Option<VarNode>,
    },
    Insert {
        output: VarNode,
        input0: VarNode,
        input1: VarNode,
        /// Must be a constant
        position: VarNode,
        /// Must be a constant
        size: VarNode,
    },
    Extract {
        output: VarNode,
        input0: VarNode,
        /// Must be a constant
        position: VarNode,
        /// Must be a constant
        size: VarNode,
    },
    PopCount {
        input: VarNode,
        output: VarNode,
    },
    LzCount {
        output: VarNode,
        input: VarNode,
    },
}

impl PcodeOperation {
    pub fn opcode(&self) -> OpCode {
        OpCode::from(self)
    }
    pub fn terminates_block(&self) -> bool {
        matches!(
            self,
            Call { .. }
                | CallOther { .. }
                | CallInd { .. }
                | Return { .. }
                | Branch { .. }
                | BranchInd { .. }
                | CBranch { .. }
        )
    }

    pub fn has_fallthrough(&self) -> bool {
        !matches!(self, Return { .. } | Branch { .. } | BranchInd { .. })
    }

    pub fn inputs(&self) -> Vec<GeneralizedVarNode> {
        match self {
            Copy { input, .. } => {
                vec![input.into()]
            }
            Load { input, .. } => {
                vec![input.into()]
            }
            Store { input, .. } => {
                vec![input.into()]
            }
            Branch { input, .. } => {
                vec![input.into()]
            }
            CBranch { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            BranchInd { input, .. } => {
                vec![input.into()]
            }
            Call { input, .. } => {
                vec![input.into()]
            }
            CallInd { input, .. } => {
                vec![input.into()]
            }
            CallOther { inputs, .. } => inputs.iter().map(|i| i.into()).collect(),
            Return { input, .. } => {
                vec![input.into()]
            }
            IntEqual { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntNotEqual { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntSignedLess { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntSignedLessEqual { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntLess { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntLessEqual { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntSExt { input, .. } => {
                vec![input.into()]
            }
            IntZExt { input, .. } => {
                vec![input.into()]
            }
            IntAdd { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntSub { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntCarry { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntSignedCarry { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntSignedBorrow { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            Int2Comp { input, .. } => {
                vec![input.into()]
            }
            IntNegate { input, .. } => {
                vec![input.into()]
            }
            IntXor { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntAnd { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntOr { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntLeftShift { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntRightShift { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntSignedRightShift { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntMult { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntDiv { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntSignedDiv { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntRem { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            IntSignedRem { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            BoolNegate { input, .. } => {
                vec![input.into()]
            }
            BoolXor { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            BoolAnd { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            BoolOr { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            FloatEqual { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            FloatNotEqual { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            FloatLess { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            FloatLessEqual { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            FloatNaN { input, .. } => {
                vec![input.into()]
            }
            FloatAdd { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            FloatDiv { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            FloatMult { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            FloatSub { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            FloatNeg { input, .. } => {
                vec![input.into()]
            }
            FloatAbs { input, .. } => {
                vec![input.into()]
            }
            FloatSqrt { input, .. } => {
                vec![input.into()]
            }
            FloatIntToFloat { input, .. } => {
                vec![input.into()]
            }
            FloatFloatToFloat { input, .. } => {
                vec![input.into()]
            }
            FloatTrunc { input, .. } => {
                vec![input.into()]
            }
            FloatCeil { input, .. } => {
                vec![input.into()]
            }
            FloatFloor { input, .. } => {
                vec![input.into()]
            }
            FloatRound { input, .. } => {
                vec![input.into()]
            }
            MultiEqual { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            Indirect { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            Piece { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            SubPiece { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            Cast { input, .. } => {
                vec![input.into()]
            }
            PtrAdd { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            PtrSub { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            SegmentOp { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            CPoolRef { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            New { input, .. } => {
                vec![input.into()]
            }
            Insert { input0, input1, .. } => {
                vec![input0.into(), input1.into()]
            }
            Extract { input0, .. } => {
                vec![input0.into()]
            }
            PopCount { input, .. } => {
                vec![input.into()]
            }
            LzCount { input, .. } => {
                vec![input.into()]
            }
        }
    }
    pub fn output(&self) -> Option<GeneralizedVarNode> {
        match self {
            Copy { output, .. } => Some(GeneralizedVarNode::from(output)),
            Load { output, .. } => Some(GeneralizedVarNode::from(output)),
            Store { output, .. } => Some(GeneralizedVarNode::from(output)),
            Branch { .. } => None,
            CBranch { .. } => None,
            BranchInd { .. } => None,
            Call { .. } => None,
            CallInd { .. } => None,
            CallOther { output, .. } => output.clone().map(GeneralizedVarNode::from),
            Return { .. } => None,
            IntEqual { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntNotEqual { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntSignedLess { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntSignedLessEqual { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntLess { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntLessEqual { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntSExt { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntZExt { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntAdd { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntSub { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntCarry { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntSignedCarry { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntSignedBorrow { output, .. } => Some(GeneralizedVarNode::from(output)),
            Int2Comp { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntNegate { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntXor { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntAnd { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntOr { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntLeftShift { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntRightShift { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntSignedRightShift { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntMult { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntDiv { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntSignedDiv { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntRem { output, .. } => Some(GeneralizedVarNode::from(output)),
            IntSignedRem { output, .. } => Some(GeneralizedVarNode::from(output)),
            BoolNegate { output, .. } => Some(GeneralizedVarNode::from(output)),
            BoolXor { output, .. } => Some(GeneralizedVarNode::from(output)),
            BoolAnd { output, .. } => Some(GeneralizedVarNode::from(output)),
            BoolOr { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatEqual { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatNotEqual { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatLess { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatLessEqual { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatNaN { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatAdd { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatDiv { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatMult { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatSub { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatNeg { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatAbs { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatSqrt { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatIntToFloat { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatFloatToFloat { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatTrunc { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatCeil { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatFloor { output, .. } => Some(GeneralizedVarNode::from(output)),
            FloatRound { output, .. } => Some(GeneralizedVarNode::from(output)),
            MultiEqual { output, .. } => Some(GeneralizedVarNode::from(output)),
            Indirect { output, .. } => Some(GeneralizedVarNode::from(output)),
            Piece { output, .. } => Some(GeneralizedVarNode::from(output)),
            SubPiece { output, .. } => Some(GeneralizedVarNode::from(output)),
            Cast { output, .. } => Some(GeneralizedVarNode::from(output)),
            PtrAdd { output, .. } => Some(GeneralizedVarNode::from(output)),
            PtrSub { output, .. } => Some(GeneralizedVarNode::from(output)),
            SegmentOp { output, .. } => Some(GeneralizedVarNode::from(output)),
            CPoolRef { output, .. } => Some(GeneralizedVarNode::from(output)),
            New { output, .. } => Some(GeneralizedVarNode::from(output)),
            Insert { output, .. } => Some(GeneralizedVarNode::from(output)),
            Extract { output, .. } => Some(GeneralizedVarNode::from(output)),
            PopCount { output, .. } => Some(GeneralizedVarNode::from(output)),
            LzCount { output, .. } => Some(GeneralizedVarNode::from(output)),
        }
    }
}

impl From<RawPcodeOp> for PcodeOperation {
    fn from(value: RawPcodeOp) -> Self {
        macro_rules! one_in {
            ($op:tt) => {
                $op {
                    input: VarNode::from(&value.inputs[0]),
                }
            };
        }

        macro_rules! one_in_indirect {
            ($op:tt) => {
                $op {
                    input: IndirectVarNode {
                        pointer_location: VarNode::from(&value.inputs[0]),
                        access_size_bytes: value.space.getAddrSize() as usize,
                        pointer_space_index: value.space.getIndex() as usize,
                    },
                }
            };
        }

        macro_rules! two_in {
            ($op:tt) => {
                $op {
                    input0: VarNode::from(&value.inputs[0]),
                    input1: VarNode::from(&value.inputs[1]),
                }
            };
        }
        macro_rules! one_in_one_out {
            ($op:tt) => {
                $op {
                    output: VarNode::from(value.output),
                    input: VarNode::from(&value.inputs[0]),
                }
            };
        }
        macro_rules! two_in_one_out {
            ($op:tt) => {
                $op {
                    output: VarNode::from(value.output),
                    input0: VarNode::from(&value.inputs[0]),
                    input1: VarNode::from(&value.inputs[1]),
                }
            };
        }
        match value.op {
            OpCode::CPUI_COPY => one_in_one_out!(Copy),
            OpCode::CPUI_LOAD => {
                let space_id = value.inputs[0].offset;
                let space = value.inputs[0]
                    .space
                    .getManager()
                    .getSpaceFromPointer(space_id);
                let output = VarNode::from(&value.output);
                Load {
                    input: IndirectVarNode {
                        pointer_space_index: space.getIndex() as usize,
                        pointer_location: VarNode::from(&value.inputs[1]),
                        access_size_bytes: output.size,
                    },
                    output: value.output.into(),
                }
            }
            OpCode::CPUI_STORE => {
                let space_id = value.inputs[0].offset;
                let space = value.inputs[0]
                    .space
                    .getManager()
                    .getSpaceFromPointer(space_id);
                let input = VarNode::from(&value.inputs[2]);
                Store {
                    output: IndirectVarNode {
                        pointer_space_index: space.getIndex() as usize,
                        pointer_location: VarNode::from(&value.inputs[1]),
                        access_size_bytes: input.size,
                    },
                    input,
                }
            }
            OpCode::CPUI_BRANCH => one_in!(Branch),
            OpCode::CPUI_CBRANCH => two_in!(CBranch),
            OpCode::CPUI_BRANCHIND => one_in_indirect!(BranchInd),
            OpCode::CPUI_CALL => one_in!(Call),
            OpCode::CPUI_CALLIND => one_in_indirect!(CallInd),
            OpCode::CPUI_CALLOTHER => {
                let output = match value.has_output {
                    true => Some(value.output.into()),
                    false => None,
                };
                //let inputs: Vec<VarNode> = Vec::with_capacity(value.inputs.len());
                let inputs: Vec<VarNode> = value.inputs.iter().map(|i| i.into()).collect();
                CallOther { inputs, output }
            }
            OpCode::CPUI_RETURN => one_in_indirect!(Return),
            OpCode::CPUI_INT_EQUAL => two_in_one_out!(IntEqual),
            OpCode::CPUI_INT_NOTEQUAL => two_in_one_out!(IntNotEqual),
            OpCode::CPUI_INT_SLESS => two_in_one_out!(IntSignedLess),
            OpCode::CPUI_INT_SLESSEQUAL => two_in_one_out!(IntSignedLessEqual),
            OpCode::CPUI_INT_LESS => two_in_one_out!(IntLess),
            OpCode::CPUI_INT_LESSEQUAL => two_in_one_out!(IntLessEqual),
            OpCode::CPUI_INT_ZEXT => one_in_one_out!(IntZExt),
            OpCode::CPUI_INT_SEXT => one_in_one_out!(IntSExt),
            OpCode::CPUI_INT_ADD => two_in_one_out!(IntAdd),
            OpCode::CPUI_INT_SUB => two_in_one_out!(IntSub),
            OpCode::CPUI_INT_CARRY => two_in_one_out!(IntCarry),
            OpCode::CPUI_INT_SCARRY => two_in_one_out!(IntSignedCarry),
            OpCode::CPUI_INT_SBORROW => two_in_one_out!(IntSignedBorrow),
            OpCode::CPUI_INT_2COMP => one_in_one_out!(Int2Comp),
            OpCode::CPUI_INT_NEGATE => one_in_one_out!(IntNegate),
            OpCode::CPUI_INT_XOR => two_in_one_out!(IntXor),
            OpCode::CPUI_INT_AND => two_in_one_out!(IntAnd),
            OpCode::CPUI_INT_OR => two_in_one_out!(IntOr),
            OpCode::CPUI_INT_LEFT => two_in_one_out!(IntLeftShift),
            OpCode::CPUI_INT_RIGHT => two_in_one_out!(IntRightShift),
            OpCode::CPUI_INT_SRIGHT => two_in_one_out!(IntSignedRightShift),
            OpCode::CPUI_INT_MULT => two_in_one_out!(IntMult),
            OpCode::CPUI_INT_DIV => two_in_one_out!(IntDiv),
            OpCode::CPUI_INT_SDIV => two_in_one_out!(IntSignedDiv),
            OpCode::CPUI_INT_REM => two_in_one_out!(IntRem),
            OpCode::CPUI_INT_SREM => two_in_one_out!(IntSignedRem),
            OpCode::CPUI_BOOL_NEGATE => one_in_one_out!(BoolNegate),
            OpCode::CPUI_BOOL_XOR => two_in_one_out!(BoolXor),
            OpCode::CPUI_BOOL_AND => two_in_one_out!(BoolAnd),
            OpCode::CPUI_BOOL_OR => two_in_one_out!(BoolOr),
            OpCode::CPUI_FLOAT_EQUAL => two_in_one_out!(FloatEqual),
            OpCode::CPUI_FLOAT_NOTEQUAL => two_in_one_out!(FloatNotEqual),
            OpCode::CPUI_FLOAT_LESS => two_in_one_out!(FloatLess),
            OpCode::CPUI_FLOAT_LESSEQUAL => two_in_one_out!(FloatLessEqual),
            OpCode::CPUI_FLOAT_NAN => one_in_one_out!(FloatNaN),
            OpCode::CPUI_FLOAT_ADD => two_in_one_out!(FloatAdd),
            OpCode::CPUI_FLOAT_DIV => two_in_one_out!(FloatDiv),
            OpCode::CPUI_FLOAT_MULT => two_in_one_out!(FloatMult),
            OpCode::CPUI_FLOAT_SUB => two_in_one_out!(FloatSub),
            OpCode::CPUI_FLOAT_NEG => one_in_one_out!(FloatNeg),
            OpCode::CPUI_FLOAT_ABS => one_in_one_out!(FloatAbs),
            OpCode::CPUI_FLOAT_SQRT => one_in_one_out!(FloatSqrt),
            OpCode::CPUI_FLOAT_INT2FLOAT => one_in_one_out!(FloatIntToFloat),
            OpCode::CPUI_FLOAT_FLOAT2FLOAT => one_in_one_out!(FloatFloatToFloat),
            OpCode::CPUI_FLOAT_TRUNC => one_in_one_out!(FloatTrunc),
            OpCode::CPUI_FLOAT_CEIL => one_in_one_out!(FloatCeil),
            OpCode::CPUI_FLOAT_FLOOR => one_in_one_out!(FloatFloor),
            OpCode::CPUI_FLOAT_ROUND => one_in_one_out!(FloatRound),
            OpCode::CPUI_MULTIEQUAL => MultiEqual {
                output: VarNode::from(value.output),
                input0: VarNode::from(&value.inputs[0]),
                input1: VarNode::from(&value.inputs[1]),
                // todo: actually parse out extra args. This never happens in raw pcode so punting for now.
                inputs: Vec::new(),
            },
            OpCode::CPUI_INDIRECT => two_in_one_out!(Indirect),
            OpCode::CPUI_PIECE => two_in_one_out!(Piece),
            OpCode::CPUI_SUBPIECE => two_in_one_out!(SubPiece),
            OpCode::CPUI_CAST => one_in_one_out!(Cast),
            OpCode::CPUI_PTRADD => PtrAdd {
                output: VarNode::from(value.output),
                input0: VarNode::from(&value.inputs[0]),
                input1: VarNode::from(&value.inputs[1]),
                input2: VarNode::from(&value.inputs[2]),
            },
            OpCode::CPUI_PTRSUB => PtrSub {
                output: VarNode::from(value.output),
                input0: VarNode::from(&value.inputs[0]),
                input1: VarNode::from(&value.inputs[1]),
            },
            OpCode::CPUI_SEGMENTOP => SegmentOp {
                output: VarNode::from(value.output),
                //todo: based on ghidra source, we likely want to extract some other piece
                // of info here from the FFI object for input0's address space instead of
                // storing the varnode
                input0: VarNode::from(&value.inputs[0]),
                input1: VarNode::from(&value.inputs[1]),
                input2: VarNode::from(&value.inputs[2]),
            },
            OpCode::CPUI_CPOOLREF => CPoolRef {
                output: VarNode::from(value.output),
                input0: VarNode::from(&value.inputs[0]),
                input1: VarNode::from(&value.inputs[1]),
                // todo: actually parse out extra args. This never happens in raw pcode so punting for now.
                inputs: Vec::new(),
            },
            OpCode::CPUI_NEW => New {
                output: VarNode::from(value.output),
                input: VarNode::from(&value.inputs[0]),
                size: value.inputs.get(1).map(VarNode::from),
            },
            OpCode::CPUI_INSERT => Insert {
                output: VarNode::from(value.output),
                input0: VarNode::from(&value.inputs[0]),
                input1: VarNode::from(&value.inputs[1]),
                position: VarNode::from(&value.inputs[2]),
                size: VarNode::from(&value.inputs[3]),
            },
            OpCode::CPUI_EXTRACT => Extract {
                output: VarNode::from(value.output),
                input0: VarNode::from(&value.inputs[0]),
                position: VarNode::from(&value.inputs[1]),
                size: VarNode::from(&value.inputs[2]),
            },
            OpCode::CPUI_POPCOUNT => one_in_one_out!(PopCount),
            OpCode::CPUI_LZCOUNT => one_in_one_out!(LzCount),
            // Sleigh should not be emitting any other values.
            _ => unreachable!(),
        }
    }
}

impl From<&PcodeOperation> for OpCode {
    fn from(value: &PcodeOperation) -> Self {
        match value {
            Copy { .. } => OpCode::CPUI_COPY,
            Load { .. } => OpCode::CPUI_LOAD,
            Store { .. } => OpCode::CPUI_STORE,
            Branch { .. } => OpCode::CPUI_BRANCH,
            CBranch { .. } => OpCode::CPUI_CBRANCH,
            BranchInd { .. } => OpCode::CPUI_BRANCHIND,
            Call { .. } => OpCode::CPUI_CALL,
            CallInd { .. } => OpCode::CPUI_CALLIND,
            CallOther { .. } => OpCode::CPUI_CALLOTHER,
            Return { .. } => OpCode::CPUI_RETURN,
            IntEqual { .. } => OpCode::CPUI_INT_EQUAL,
            IntNotEqual { .. } => OpCode::CPUI_INT_NOTEQUAL,
            IntSignedLess { .. } => OpCode::CPUI_INT_SLESS,
            IntSignedLessEqual { .. } => OpCode::CPUI_INT_SLESSEQUAL,
            IntLess { .. } => OpCode::CPUI_INT_LESS,
            IntLessEqual { .. } => OpCode::CPUI_INT_LESSEQUAL,
            IntSExt { .. } => OpCode::CPUI_INT_SEXT,
            IntZExt { .. } => OpCode::CPUI_INT_ZEXT,
            IntAdd { .. } => OpCode::CPUI_INT_ADD,
            IntSub { .. } => OpCode::CPUI_INT_SUB,
            IntCarry { .. } => OpCode::CPUI_INT_CARRY,
            IntSignedCarry { .. } => OpCode::CPUI_INT_SCARRY,
            IntSignedBorrow { .. } => OpCode::CPUI_INT_SBORROW,
            Int2Comp { .. } => OpCode::CPUI_INT_2COMP,
            IntNegate { .. } => OpCode::CPUI_INT_NEGATE,
            IntXor { .. } => OpCode::CPUI_INT_XOR,
            IntAnd { .. } => OpCode::CPUI_INT_AND,
            IntOr { .. } => OpCode::CPUI_INT_OR,
            IntLeftShift { .. } => OpCode::CPUI_INT_LEFT,
            IntRightShift { .. } => OpCode::CPUI_INT_RIGHT,
            IntSignedRightShift { .. } => OpCode::CPUI_INT_SRIGHT,
            IntMult { .. } => OpCode::CPUI_INT_MULT,
            IntDiv { .. } => OpCode::CPUI_INT_DIV,
            IntSignedDiv { .. } => OpCode::CPUI_INT_SDIV,
            IntRem { .. } => OpCode::CPUI_INT_REM,
            IntSignedRem { .. } => OpCode::CPUI_INT_SREM,
            BoolNegate { .. } => OpCode::CPUI_BOOL_NEGATE,
            BoolXor { .. } => OpCode::CPUI_BOOL_XOR,
            BoolAnd { .. } => OpCode::CPUI_BOOL_AND,
            BoolOr { .. } => OpCode::CPUI_BOOL_OR,
            FloatEqual { .. } => OpCode::CPUI_FLOAT_EQUAL,
            FloatNotEqual { .. } => OpCode::CPUI_FLOAT_NOTEQUAL,
            FloatLess { .. } => OpCode::CPUI_FLOAT_LESS,
            FloatLessEqual { .. } => OpCode::CPUI_FLOAT_LESSEQUAL,
            FloatNaN { .. } => OpCode::CPUI_FLOAT_NAN,
            FloatAdd { .. } => OpCode::CPUI_FLOAT_ADD,
            FloatDiv { .. } => OpCode::CPUI_FLOAT_DIV,
            FloatMult { .. } => OpCode::CPUI_FLOAT_MULT,
            FloatSub { .. } => OpCode::CPUI_FLOAT_SUB,
            FloatNeg { .. } => OpCode::CPUI_FLOAT_NEG,
            FloatAbs { .. } => OpCode::CPUI_FLOAT_ABS,
            FloatSqrt { .. } => OpCode::CPUI_FLOAT_SQRT,
            FloatIntToFloat { .. } => OpCode::CPUI_FLOAT_INT2FLOAT,
            FloatFloatToFloat { .. } => OpCode::CPUI_FLOAT_FLOAT2FLOAT,
            FloatTrunc { .. } => OpCode::CPUI_FLOAT_TRUNC,
            FloatCeil { .. } => OpCode::CPUI_FLOAT_CEIL,
            FloatFloor { .. } => OpCode::CPUI_FLOAT_FLOOR,
            FloatRound { .. } => OpCode::CPUI_FLOAT_ROUND,
            MultiEqual { .. } => OpCode::CPUI_MULTIEQUAL,
            Indirect { .. } => OpCode::CPUI_INDIRECT,
            Piece { .. } => OpCode::CPUI_PIECE,
            SubPiece { .. } => OpCode::CPUI_SUBPIECE,
            Cast { .. } => OpCode::CPUI_CAST,
            PtrAdd { .. } => OpCode::CPUI_PTRADD,
            PtrSub { .. } => OpCode::CPUI_PTRSUB,
            SegmentOp { .. } => OpCode::CPUI_SEGMENTOP,
            CPoolRef { .. } => OpCode::CPUI_CPOOLREF,
            New { .. } => OpCode::CPUI_NEW,
            Insert { .. } => OpCode::CPUI_INSERT,
            Extract { .. } => OpCode::CPUI_EXTRACT,
            PopCount { .. } => OpCode::CPUI_POPCOUNT,
            LzCount { .. } => OpCode::CPUI_LZCOUNT,
        }
    }
}

impl Display for PcodeOperation {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        if let Some(out) = self.output() {
            write!(f, "{out} = ")?;
        }
        write!(f, "{} ", self.opcode())?;
        let i: Vec<_> = self.inputs().iter().map(|ff| format!("{ff}")).collect();
        write!(f, "{}", i.join(", "))
    }
}

impl LowerHex for PcodeOperation {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        if let Some(out) = self.output() {
            write!(f, "{out:x} = ")?;
        }
        write!(f, "{} ", self.opcode())?;
        let i: Vec<_> = self.inputs().iter().map(|ff| format!("{ff:x}")).collect();
        write!(f, "{}", i.join(", "))
    }
}

impl Display for OpCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = format!("{:?}", self);
        write!(f, "{}", s.as_str().get(5..).unwrap())
    }
}
