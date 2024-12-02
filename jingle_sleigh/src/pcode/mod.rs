pub mod branch;
pub mod display;

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

use crate::error::JingleSleighError;
use crate::ffi::instruction::bridge::RawPcodeOp;
pub use crate::ffi::opcode::OpCode;
use crate::varnode::{IndirectVarNode, VarNode};
use crate::{GeneralizedVarNode, RegisterManager};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
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
