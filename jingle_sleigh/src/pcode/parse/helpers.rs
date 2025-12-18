use pest::iterators::Pair;
use crate::{IndirectVarNode, JingleSleighError, SleighArchInfo, VarNode};
use crate::parse::Rule;

pub fn const_to_varnode(s: &str, info: &SleighArchInfo) -> Result<VarNode, JingleSleighError> {
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
/// Returns a parsed IndirectVarNode with the pointer space and pointer location set.
/// The access size is left as 0 for the caller to decide.
pub fn parse_reference_pair(
    pair: Pair<Rule>,
    info: &SleighArchInfo,
) -> Result<IndirectVarNode, JingleSleighError> {
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
    Ok(IndirectVarNode {
        pointer_space_index: space.index,
        pointer_location,
        access_size_bytes: 0,
    })
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