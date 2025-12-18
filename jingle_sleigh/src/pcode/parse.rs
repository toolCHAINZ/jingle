use pest::{
    Parser,
    iterators::{Pair, Pairs},
};
use pest_derive::Parser;
use tracing::warn;

use crate::{IndirectVarNode, JingleSleighError, PcodeOperation, SleighArchInfo, VarNode};

#[derive(Parser)]
#[grammar = "pcode/grammar.pest"]
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

fn const_to_varnode(s: &str, info: &SleighArchInfo) -> Result<VarNode, JingleSleighError> {
    let s = s.trim();
    let radix = if s.starts_with("0x") { 16 } else { 10 };
    let s = s.strip_prefix("0x").unwrap_or(s);
    let offset = u64::from_str_radix(s, radix).map_err(|_| {
        JingleSleighError::PcodeParseValidation(format!("Invalid const literal: {}", s))
    })?;
    let space = info
        .get_space_by_name("const")
        .ok_or(JingleSleighError::PcodeParseValidation(
            "Missing const space in arch info".to_string(),
        ))?;
    // We don't have a size for these plain const tokens in the grammar; choose 0 to indicate "constant"
    Ok(VarNode {
        offset,
        space_index: space.index,
        size: 0,
    })
}

/// Parse a reference pair using grammar pairs (reference = space ~ "(" ~ varnode ~ ")")
/// Returns the pointer space index and the parsed pointer VarNode; the caller decides access size.
fn parse_reference_pair(
    pair: Pair<Rule>,
    info: &SleighArchInfo,
) -> Result<(usize, VarNode), JingleSleighError> {
    // Walk the inner pairs produced by the `reference` rule:
    // expected sequence: Rule::space, Rule::varnode
    let mut inner = pair.into_inner();
    let space_pair = inner.next().ok_or(JingleSleighError::PcodeParseValidation(
        "Missing space in reference".to_string(),
    ))?;
    if space_pair.as_rule() != Rule::space {
        return Err(JingleSleighError::PcodeParseValidation(format!(
            "Expected space in reference, got {:?}",
            space_pair.as_rule()
        )));
    }
    let varnode_pair = inner.next().ok_or(JingleSleighError::PcodeParseValidation(
        "Missing varnode in reference".to_string(),
    ))?;
    if varnode_pair.as_rule() != Rule::varnode {
        return Err(JingleSleighError::PcodeParseValidation(format!(
            "Expected varnode in reference, got {:?}",
            varnode_pair.as_rule()
        )));
    }

    let space_name = space_pair.as_str();
    let pointer_location = parse_varnode(varnode_pair, info)?;
    let space =
        info.get_space_by_name(space_name)
            .ok_or(JingleSleighError::PcodeParseValidation(format!(
                "Invalid space: {}",
                space_name
            )))?;
    Ok((space.index, pointer_location))
}

pub fn parse_pcode(
    pairs: Pairs<Rule>,
    info: &SleighArchInfo,
) -> Result<PcodeOperation, JingleSleighError> {
    for pair in pairs {
        dbg!(&pair);
        match pair.as_rule() {
            Rule::COPY => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input = parse_varnode(pairs[1].clone(), info)?;
                return Ok(PcodeOperation::Copy { input, output });
            }
            Rule::LOAD => {
                let pairs: Vec<_> = pair.into_inner().collect();
                // pairs[0] = output varnode, pairs[1] = reference
                let output = parse_varnode(pairs[0].clone(), info)?;
                let (pointer_space_index, pointer_location) =
                    parse_reference_pair(pairs[1].clone(), info)?;
                let input = IndirectVarNode {
                    pointer_space_index,
                    pointer_location,
                    access_size_bytes: output.size,
                };
                return Ok(PcodeOperation::Load { input, output });
            }
            Rule::STORE => {
                let pairs: Vec<_> = pair.into_inner().collect();
                // pairs[0] = reference, pairs[1] = varnode to store
                let (pointer_space_index, pointer_location) =
                    parse_reference_pair(pairs[0].clone(), info)?;
                let input = parse_varnode(pairs[1].clone(), info)?;
                let output = IndirectVarNode {
                    pointer_space_index,
                    pointer_location,
                    access_size_bytes: input.size,
                };
                return Ok(PcodeOperation::Store { output, input });
            }
            Rule::BRANCH => {
                let mut inner = pair.into_inner();
                let dest = inner.next().unwrap();
                match dest.as_rule() {
                    Rule::varnode => {
                        let input = parse_varnode(dest, info)?;
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
                let input0 = parse_varnode(dest_pair, info)?;
                let input1 = parse_varnode(pairs[1].clone(), info)?;
                return Ok(PcodeOperation::CBranch { input0, input1 });
            }
            Rule::BRANCHIND => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let vn = parse_varnode(pairs[0].clone(), info)?;
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
                let dest = parse_varnode(dest_pair, info)?;
                return Ok(PcodeOperation::Call {
                    dest,
                    args: vec![],
                    call_info: None,
                });
            }
            Rule::CALLIND => {
                let mut inner = pair.into_inner();
                let p = inner.next().unwrap();
                let vn = parse_varnode(p, info)?;
                let input = IndirectVarNode {
                    pointer_space_index: vn.space_index,
                    pointer_location: vn,
                    access_size_bytes: 0,
                };
                return Ok(PcodeOperation::CallInd { input });
            }
            Rule::RETURN => {
                let mut inner = pair.into_inner();
                let p = inner.next().unwrap();
                let vn = parse_varnode(p, info)?;
                let input = IndirectVarNode {
                    pointer_space_index: vn.space_index,
                    pointer_location: vn,
                    access_size_bytes: 0,
                };
                return Ok(PcodeOperation::Return { input });
            }
            Rule::PIECE => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::Piece {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::SUBPIECE => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::SubPiece {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::POPCOUNT => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input = parse_varnode(pairs[1].clone(), info)?;
                return Ok(PcodeOperation::PopCount { input, output });
            }
            Rule::LZCOUNT => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input = parse_varnode(pairs[1].clone(), info)?;
                return Ok(PcodeOperation::LzCount { output, input });
            }
            // integer comparisons & casts
            Rule::INT_EQUAL => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntEqual {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_NOTEQUAL => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntNotEqual {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_LESS => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntLess {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_SLESS => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntSignedLess {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_LESSEQUAL => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntLessEqual {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_SLESSEQUAL => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntSignedLessEqual {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_ZEXT => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input = parse_varnode(pairs[1].clone(), info)?;
                return Ok(PcodeOperation::IntZExt { input, output });
            }
            Rule::INT_SEXT => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input = parse_varnode(pairs[1].clone(), info)?;
                return Ok(PcodeOperation::IntSExt { input, output });
            }
            // arithmetic / logical binary ops
            Rule::INT_ADD => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntAdd {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_SUB => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntSub {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_CARRY => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntCarry {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_SCARRY => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntSignedCarry {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_SBORROW => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntSignedBorrow {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_2COMP => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input = parse_varnode(pairs[1].clone(), info)?;
                return Ok(PcodeOperation::Int2Comp { output, input });
            }
            Rule::INT_NEGATE => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input = parse_varnode(pairs[1].clone(), info)?;
                return Ok(PcodeOperation::IntNegate { output, input });
            }
            Rule::INT_XOR => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntXor {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_AND => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntAnd {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_OR => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntOr {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_LEFT => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntLeftShift {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_RIGHT => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntRightShift {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_SRIGHT => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntSignedRightShift {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_MULT => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntMult {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_DIV => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntDiv {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_SDIV => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntSignedDiv {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_REM => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntRem {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::INT_SREM => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::IntSignedRem {
                    output,
                    input0,
                    input1,
                });
            }
            // boolean ops
            Rule::BOOL_NEGATE => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input = parse_varnode(pairs[1].clone(), info)?;
                return Ok(PcodeOperation::BoolNegate { output, input });
            }
            Rule::BOOL_XOR => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::BoolXor {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::BOOL_AND => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::BoolAnd {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::BOOL_OR => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::BoolOr {
                    output,
                    input0,
                    input1,
                });
            }
            // floating point ops (handle common forms)
            Rule::FLOAT_EQUAL => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::FloatEqual {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::FLOAT_NOTEQUAL => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::FloatNotEqual {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::FLOAT_LESS => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::FloatLess {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::FLOAT_LESSEQUAL => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::FloatLessEqual {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::FLOAT_NAN => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input = parse_varnode(pairs[1].clone(), info)?;
                return Ok(PcodeOperation::FloatNaN { output, input });
            }
            Rule::FLOAT_ADD => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::FloatAdd {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::FLOAT_SUB => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::FloatSub {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::FLOAT_MULT => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::FloatMult {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::FLOAT_DIV => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input0 = parse_varnode(pairs[1].clone(), info)?;
                let input1 = parse_varnode(pairs[2].clone(), info)?;
                return Ok(PcodeOperation::FloatDiv {
                    output,
                    input0,
                    input1,
                });
            }
            Rule::FLOAT_NEG => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input = parse_varnode(pairs[1].clone(), info)?;
                return Ok(PcodeOperation::FloatNeg { output, input });
            }
            Rule::FLOAT_ABS => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let output = parse_varnode(pairs[0].clone(), info)?;
                let input = parse_varnode(pairs[1].clone(), info)?;
                return Ok(PcodeOperation::FloatAbs { output, input });
            }
            // many other pcode rules are possible; fall through to unreachable to catch unhandled ones
            a => {
                // For debugging, print the rule we hit.
                dbg!(a);
                return Err(JingleSleighError::PcodeParseValidation(format!(
                    "Unhandled pcode rule in parser: {:?}",
                    a
                )));
            }
        }
    }
    unreachable!()
}

pub fn parse_varnode(
    pair: Pair<Rule>,
    info: &SleighArchInfo,
) -> Result<VarNode, JingleSleighError> {
    let mut loc: Option<(String, u64)> = None;
    let mut size: Option<usize> = None;
    let mut reg: Option<String> = None;

    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::location => {
                let new_loc = parse_varnode_location(pair)?;
                loc = Some(new_loc);
            }
            Rule::size => size = Some(usize::from_str_radix(pair.as_str(), 10).unwrap()),
            Rule::register => {
                reg = Some(pair.as_str().to_string());
            }
            _ => unreachable!(),
        }
    }
    if let Some((loc, size)) = loc.and_then(|l| size.map(|s| (l, s))) {
        let space =
            info.get_space_by_name(&loc.0)
                .ok_or(JingleSleighError::PcodeParseValidation(format!(
                    "Invalid space: {}",
                    loc.0
                )))?;
        return Ok(VarNode {
            offset: loc.1,
            space_index: space.index,
            size: size,
        });
    } else {
        let reg = reg.unwrap();
        return info
            .register(&reg)
            .cloned()
            .ok_or(JingleSleighError::PcodeParseValidation(format!(
                "Invalid register: {}",
                reg
            )));
    }
}

pub fn parse_varnode_location(pair: Pair<Rule>) -> Result<(String, u64), JingleSleighError> {
    dbg!(&pair);
    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::temporary => {
                let s = pair.as_span().as_str();
                return Ok((
                    "unique".to_string(),
                    u64::from_str_radix(&s[2..], 16).unwrap(),
                ));
            }
            Rule::r#const => {
                let s = pair.as_span().as_str();
                let radix = if s.starts_with("0x") { 16 } else { 10 };
                let s = s.strip_prefix("0x").unwrap_or(&s);
                return Ok(("const".to_string(), u64::from_str_radix(s, radix).unwrap()));
            }
            Rule::memory => {
                let pairs: Vec<_> = pair.into_inner().collect();
                let space = pairs[0].as_str().to_string();
                let offset = pairs[1].as_str();
                let radix = if offset.starts_with("0x") { 16 } else { 10 };
                let offset = u64::from_str_radix(offset, radix).unwrap();
                return Ok((space, offset));
            }
            _ => unreachable!(),
        }
    }
    unreachable!()
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
                input: "0x10:1 = COPY 0x11:1\n",
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
