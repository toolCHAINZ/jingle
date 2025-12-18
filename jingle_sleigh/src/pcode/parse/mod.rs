use pest::{
    Parser,
    iterators::{Pair, Pairs},
};
use pest_derive::Parser;
use tracing::warn;

use crate::{IndirectVarNode, JingleSleighError, PcodeOperation, SleighArchInfo, VarNode};

mod helpers;

#[derive(Parser)]
#[grammar = "pcode/parse/grammar.pest"]
pub struct PcodeParser;

pub fn parse_program<T: AsRef<str>>(
    s: T,
    info: SleighArchInfo,
) -> Result<Vec<PcodeOperation>, JingleSleighError> {
    let pairs = PcodeParser::parse(Rule::PROGRAM, s.as_ref())?;
    let mut ops = vec![];
    for pair in pairs {
        match pair.as_rule() {
            Rule::PCODE => {
                let op = parse_pcode(pair.into_inner(), &info)?;
                ops.push(op);
            }
            Rule::LABEL => {
                warn!("Attempting to parse p-code program with textual labels; this code is brittle and likely does
                    not work in the general case. Please ensure the parsed p-code's control flow matches what you expect.")
            }
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }
    Ok(ops)
}

pub fn parse_pcode(
    mut pairs: Pairs<Rule>,
    info: &SleighArchInfo,
) -> Result<PcodeOperation, JingleSleighError> {
    let pair = pairs.next().unwrap();

    macro_rules! parse_unop {
        ($rule:ident) => {{
            let pairs: Vec<_> = pair.into_inner().collect();
            let output = helpers::parse_varnode(pairs[0].clone(), info)?;
            let input = helpers::parse_varnode(pairs[1].clone(), info)?;
            Ok(PcodeOperation::$rule {
                output,
                input,
            })
        }};
    }
    macro_rules! parse_binop {
        ($rule:ident) => {{
            let pairs: Vec<_> = pair.into_inner().collect();
            let output = helpers::parse_varnode(pairs[0].clone(), info)?;
            let input0 = helpers::parse_varnode(pairs[1].clone(), info)?;
            let input1 = helpers::parse_varnode(pairs[2].clone(), info)?;
            Ok(PcodeOperation::$rule {
                output,
                input0,
                input1,
            })
        }};
    }

    match pair.as_rule() {
        Rule::COPY => {
            let pairs: Vec<_> = pair.into_inner().collect();
            let output = helpers::parse_varnode(pairs[0].clone(), info)?;
            let input = helpers::parse_varnode(pairs[1].clone(), info)?;
            return Ok(PcodeOperation::Copy { input, output });
        }
        Rule::LOAD => {
            let pairs: Vec<_> = pair.into_inner().collect();
            // pairs[0] = output varnode, pairs[1] = reference
            let output = helpers::parse_varnode(pairs[0].clone(), info)?;
            let mut input = helpers::parse_reference_pair(pairs[1].clone(), info)?;
            input.access_size_bytes = output.size;
            return Ok(PcodeOperation::Load { input, output });
        }
        Rule::STORE => {
            let pairs: Vec<_> = pair.into_inner().collect();
            // pairs[0] = reference, pairs[1] = varnode to store
            let mut output = helpers::parse_reference_pair(pairs[0].clone(), info)?;
            let input = helpers::parse_varnode(pairs[1].clone(), info)?;
            output.access_size_bytes = input.size;
            return Ok(PcodeOperation::Store { output, input });
        }
        Rule::BRANCH => {
            let mut inner = pair.into_inner();
            let dest = inner.next().unwrap();
            match dest.as_rule() {
                Rule::varnode => {
                    let input = helpers::parse_varnode(dest, info)?;
                    return Ok(PcodeOperation::Branch { input });
                }
                Rule::LABEL => {
                    return Err(JingleSleighError::PcodeParseValidation(
                        "BRANCH to textual LABEL not supported".to_string(),
                    ));
                }
                _ => unreachable!(),
            }
        }
        Rule::CBRANCH => {
            let pairs: Vec<_> = pair.into_inner().collect();
            // pairs[0] = branch_dest (varnode or label), pairs[1] = varnode condition
            let dest_pair = pairs[0].clone();
            if dest_pair.as_rule() != Rule::varnode {
                return Err(JingleSleighError::PcodeParseValidation(
                    "CBRANCH with non-varnode destination not supported".to_string(),
                ));
            }
            let input0 = helpers::parse_varnode(dest_pair, info)?;
            let input1 = helpers::parse_varnode(pairs[1].clone(), info)?;
            return Ok(PcodeOperation::CBranch { input0, input1 });
        }
        Rule::BRANCHIND => {
            let pairs: Vec<_> = pair.into_inner().collect();
            let vn = helpers::parse_varnode(pairs[0].clone(), info)?;
            let input = IndirectVarNode {
                pointer_space_index: vn.space_index,
                pointer_location: vn.clone(),
                access_size_bytes: vn.size,
            };
            return Ok(PcodeOperation::BranchInd { input });
        }
        Rule::CALL => {
            let mut inner = pair.into_inner();
            let dest_pair = inner.next().unwrap();
            let dest = helpers::parse_varnode(dest_pair, info)?;
            return Ok(PcodeOperation::Call {
                dest,
                args: vec![],
                call_info: None,
            });
        }
        Rule::CALLIND => {
            let mut inner = pair.into_inner();
            let p = inner.next().unwrap();
            let vn = helpers::parse_varnode(p, info)?;
            let input = IndirectVarNode {
                pointer_space_index: info.default_code_space_index(),
                pointer_location: vn,
                access_size_bytes: 0,
            };
            return Ok(PcodeOperation::CallInd { input });
        }
        Rule::CALLOTHER => {
            // Grammar (updated): (varnode "=")? "CALLOTHER" callother_operation callother_args
            // callother_operation = varnode | string
            // callother_args = ("," ~ varnode)*

            let mut inner = pair.into_inner();
            let mut output: Option<VarNode> = None;

            // First inner could be the optional output varnode or the callother_operation
            let first = inner.next().ok_or(JingleSleighError::PcodeParseValidation(
                "Empty CALLOTHER".to_string(),
            ))?;

            let op_pair: Pair<Rule>;
            if first.as_rule() == Rule::varnode {
                // optional output present
                output = Some(helpers::parse_varnode(first.clone(), info)?);
                op_pair = inner.next().ok_or(JingleSleighError::PcodeParseValidation(
                    "Missing callother operation".to_string(),
                ))?;
            } else {
                op_pair = first;
            }

            if op_pair.as_rule() != Rule::callother_operation {
                return Err(JingleSleighError::PcodeParseValidation(format!(
                    "Expected callother_operation, got {:?}",
                    op_pair.as_rule()
                )));
            }

            let mut inputs: Vec<VarNode> = Vec::new();

            // callother_operation may contain either a varnode or a string
            for op_inner in op_pair.into_inner() {
                match op_inner.as_rule() {
                    Rule::varnode => {
                        inputs.push(helpers::parse_varnode(op_inner, info)?);
                    }
                    Rule::string => {
                        // string includes quotes from grammar; strip surrounding quotes
                        let s = op_inner.as_str();
                        let name = s.trim_matches('\"');
                        if let Some(idx) = info.userop_index(name) {
                            // represent the op index as a const varnode
                            inputs.push(helpers::const_to_varnode(&format!("{}", idx), info)?);
                        } else {
                            return Err(JingleSleighError::PcodeParseValidation(format!(
                                "Unknown CALLOTHER operator: {}",
                                name
                            )));
                        }
                    }
                    a => {
                        dbg!(a);
                        return Err(JingleSleighError::PcodeParseValidation(
                            "Unexpected token in callother_operation".to_string(),
                        ));
                    }
                }
            }

            // Remaining inner pair is callother_args, if present
            if let Some(args_pair) = inner.next() {
                if args_pair.as_rule() != Rule::callother_args {
                    return Err(JingleSleighError::PcodeParseValidation(format!(
                        "Expected callother_args, got {:?}",
                        args_pair.as_rule()
                    )));
                }
                for arg in args_pair.into_inner() {
                    if arg.as_rule() != Rule::varnode {
                        return Err(JingleSleighError::PcodeParseValidation(
                            "Expected varnode in callother_args".to_string(),
                        ));
                    }
                    inputs.push(helpers::parse_varnode(arg, info)?);
                }
            }

            return Ok(PcodeOperation::CallOther {
                output,
                inputs,
                call_info: None,
            });
        }
        Rule::RETURN => {
            let mut inner = pair.into_inner();
            let p = inner.next().unwrap();
            let vn = helpers::parse_varnode(p, info)?;
            let input = IndirectVarNode {
                pointer_space_index: info.default_code_space_index(),
                pointer_location: vn,
                access_size_bytes: 0,
            };
            return Ok(PcodeOperation::Return { input });
        }
        Rule::PIECE => parse_binop!(Piece),
        Rule::SUBPIECE => parse_binop!(SubPiece),
        Rule::POPCOUNT => {
            let pairs: Vec<_> = pair.into_inner().collect();
            let output = helpers::parse_varnode(pairs[0].clone(), info)?;
            let input = helpers::parse_varnode(pairs[1].clone(), info)?;
            return Ok(PcodeOperation::PopCount { input, output });
        }
        Rule::LZCOUNT => parse_unop!(LzCount),
        // integer comparisons & casts
        Rule::INT_EQUAL => parse_binop!(IntEqual),
        Rule::INT_NOTEQUAL => parse_binop!(IntNotEqual),
        Rule::INT_LESS => parse_binop!(IntLess),
        Rule::INT_SLESS => parse_binop!(IntSignedLess),
        Rule::INT_LESSEQUAL => parse_binop!(IntLessEqual),
        Rule::INT_SLESSEQUAL => parse_binop!(IntSignedLessEqual),
        Rule::INT_ZEXT => {
            let pairs: Vec<_> = pair.into_inner().collect();
            let output = helpers::parse_varnode(pairs[0].clone(), info)?;
            let input = helpers::parse_varnode(pairs[1].clone(), info)?;
            return Ok(PcodeOperation::IntZExt { input, output });
        }
        Rule::INT_SEXT => {
            let pairs: Vec<_> = pair.into_inner().collect();
            let output = helpers::parse_varnode(pairs[0].clone(), info)?;
            let input = helpers::parse_varnode(pairs[1].clone(), info)?;
            return Ok(PcodeOperation::IntSExt { input, output });
        }
        // arithmetic / logical binary ops
        Rule::INT_ADD => parse_binop!(IntAdd),
        Rule::INT_SUB => parse_binop!(IntSub),
        Rule::INT_CARRY => parse_binop!(IntCarry),
        Rule::INT_SCARRY => parse_binop!(IntSignedCarry),
        Rule::INT_SBORROW => parse_binop!(IntSignedBorrow),
        Rule::INT_2COMP => parse_unop!(Int2Comp),
        Rule::INT_NEGATE => parse_unop!(IntNegate),
        Rule::INT_XOR => parse_binop!(IntXor),
        Rule::INT_AND => parse_binop!(IntAnd),
        Rule::INT_OR => parse_binop!(IntOr),
        Rule::INT_LEFT => parse_binop!(IntLeftShift),
        Rule::INT_RIGHT => parse_binop!(IntRightShift),
        Rule::INT_SRIGHT => parse_binop!(IntSignedRightShift),
        Rule::INT_MULT => parse_binop!(IntMult),
        Rule::INT_DIV => parse_binop!(IntDiv),
        Rule::INT_SDIV => parse_binop!(IntSignedDiv),
        Rule::INT_REM => parse_binop!(IntRem),
        Rule::INT_SREM => parse_binop!(IntSignedRem),
        // boolean ops
        Rule::BOOL_NEGATE => parse_unop!(BoolNegate),
        Rule::BOOL_XOR => parse_binop!(BoolXor),
        Rule::BOOL_AND => parse_binop!(BoolAnd),
        Rule::BOOL_OR => parse_binop!(BoolOr),
        // floating point ops (handle common forms)
        Rule::FLOAT_EQUAL => parse_binop!(FloatEqual),
        Rule::FLOAT_NOTEQUAL => parse_binop!(FloatNotEqual),
        Rule::FLOAT_LESS => parse_binop!(FloatLess),
        Rule::FLOAT_LESSEQUAL => parse_binop!(FloatLessEqual),
        Rule::FLOAT_NAN => parse_unop!(FloatNaN),
        Rule::FLOAT_ADD => parse_binop!(FloatAdd),
        Rule::FLOAT_SUB => parse_binop!(FloatSub),
        Rule::FLOAT_MULT => parse_binop!(FloatMult),
        Rule::FLOAT_DIV => parse_binop!(FloatDiv),
        Rule::FLOAT_NEG => parse_unop!(FloatNeg),
        Rule::FLOAT_ABS => parse_unop!(FloatAbs),
        Rule::FLOAT_SQRT => parse_unop!(FloatSqrt),
        Rule::FLOAT_CEIL => parse_unop!(FloatCeil),
        Rule::FLOAT_FLOOR => parse_unop!(FloatFloor),
        Rule::FLOAT_ROUND => parse_unop!(FloatRound),
        Rule::INT2FLOAT => parse_unop!(Int2Float),
        Rule::FLOAT2FLOAT => parse_unop!(Float2Float),
        Rule::TRUNC => parse_unop!(FloatTrunc),
        // many other pcode rules are possible; fall through to unreachable to catch unhandled ones
        a => {
            // For debugging, print the rule we hit.
            return Err(JingleSleighError::PcodeParseValidation(format!(
                "Unhandled pcode rule in parser: {:?}",
                a
            )));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VarNode;
    use crate::context::SleighContextBuilder;
    use crate::tests::SLEIGH_ARCH;

    fn make_info() -> SleighArchInfo {
        // Initialize a real sleigh context (as other tests in this crate do) and take its arch info.
        // The path here mirrors other tests in the repo which expect a local Ghidra checkout at this path.
        let ctx_builder =
            SleighContextBuilder::load_ghidra_installation("/Applications/ghidra").unwrap();
        let sleigh = ctx_builder.build(SLEIGH_ARCH).unwrap();
        sleigh.arch_info().clone()
    }

    #[test]
    fn parameterized_parse_pcode_copy() {
        let info = make_info();

        struct Case {
            input: &'static str,
            expected: Vec<PcodeOperation>,
        }

        let cases = vec![
            Case {
                // format: <const_or_varnode>:<size> = COPY <const_or_varnode>:<size>
                input: "0:1 = CALLOTHER \"segment\", 1:1\n",
                expected: vec![PcodeOperation::Copy {
                    input: VarNode {
                        space_index: 0,
                        offset: 0x11,
                        size: 1,
                    },
                    output: VarNode {
                        space_index: 0,
                        offset: 0x10,
                        size: 1,
                    },
                }],
            },
            Case {
                // temporary style varnode (hex) - parser should accept temporaries like $U1 as well
                input: "$U8000:8 = COPY RAX\n",
                expected: vec![PcodeOperation::Copy {
                    input: VarNode {
                        space_index: 4,
                        offset: 0x0,
                        size: 8,
                    },
                    output: VarNode {
                        space_index: 2,
                        offset: 0x8000, // NOTE: depending on parser semantics this may map differently; adjust when implementing
                        size: 8,
                    },
                }],
            },
        ];

        for case in cases {
            let got = parse_program(case.input, info.clone()).expect("parsing pcode");
            assert_eq!(got, case.expected, "source=\n{}", case.input);
        }
    }
}
