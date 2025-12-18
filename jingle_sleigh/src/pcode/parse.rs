use pest::{
    Parser,
    iterators::{Pair, Pairs},
};
use pest_derive::Parser;
use tracing::warn;

use crate::{JingleSleighError, PcodeOperation, SleighArchInfo, VarNode};

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
            a => {
                unreachable!()
            }
        }
    }
    todo!()
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
