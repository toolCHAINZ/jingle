/*!
A small pest-based parser for a tiny subset of pcode used in tests.

This file provides:
- `PcodeParser` derived from `pcode/grammar.pest`
- A tiny AST (`Varnode`, `CopyOp`)
- Simple helper functions to parse varnodes and `COPY` operations
- Unit tests that validate parsing behavior

Note: the grammar file `pcode/grammar.pest` lives at `src/pcode/grammar.pest`.
*/

use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

/// Generated parser from `grammar.pest`
#[derive(Parser)]
#[grammar = "pcode/grammar.pest"]
pub struct PcodeParser;

/// Tiny AST representing a pcode varnode used in tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Varnode {
    /// Temporary varnode of form `$U<hex>:<size>`.
    Temporary { id: u64, size: usize },
    /// Constant varnode of form `0x<hex>:<size>` or `<dec>:<size>`.
    Const { value: u64, size: usize },
    /// Register varnode - a single ASCII character.
    Register { name: char },
    /// Memory access like `[SPACE]0xA:4` per grammar: `[` space `]` const
    Memory {
        space: String,
        value: u64,
        size: usize,
    },
}

/// COPY operation AST: `varnode "=" "COPY" varnode`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyOp {
    pub dest: Varnode,
    pub src: Varnode,
}

/// Parse a varnode string (using the parser) into a `Varnode`.
///
/// Returns `Err(String)` with a human-friendly message on parse errors.
pub fn parse_varnode(s: &str) -> Result<Varnode, String> {
    let mut pairs =
        PcodeParser::parse(Rule::varnode, s).map_err(|e| format!("parse error: {}", e))?;
    let pair = pairs
        .next()
        .ok_or_else(|| "no varnode produced".to_string())?;
    varnode_from_pair(pair)
}

/// Parse a COPY operation string into a `CopyOp`.
///
/// Example: `a=COPY$U4:8`
pub fn parse_copy(s: &str) -> Result<CopyOp, String> {
    let mut pairs = PcodeParser::parse(Rule::COPY, s).map_err(|e| format!("parse error: {}", e))?;
    let pair = pairs.next().ok_or_else(|| "no COPY produced".to_string())?;
    let mut inner = pair.into_inner();

    // The grammar is `varnode ~ "=" ~ "COPY" ~ varnode`.
    // Only the `varnode` rule instances produce pairs; get two varnode pairs.
    let left_pair = inner
        .next()
        .ok_or_else(|| "missing left varnode".to_string())?;
    let right_pair = inner
        .next()
        .ok_or_else(|| "missing right varnode".to_string())?;

    let dest = varnode_from_pair(left_pair)?;
    let src = varnode_from_pair(right_pair)?;
    Ok(CopyOp { dest, src })
}

fn varnode_from_pair(pair: Pair<Rule>) -> Result<Varnode, String> {
    // Some rule names (like `const`) are Rust keywords or otherwise may not map to the
    // expected enum variant name depending on how the pest derive generated `Rule`.
    // To avoid relying on variant names, match on the textual rule name instead.
    let rule_name = format!("{:?}", pair.as_rule());
    match rule_name.as_str() {
        "temporary" => {
            // temporary = { "$U" ~ HEX_DIGIT ~ ":" ~ size }
            let s = pair.as_str();
            if !s.starts_with("$U") {
                return Err(format!("invalid temporary: {}", s));
            }
            let rest = &s[2..];
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(format!("temporary missing size: {}", s));
            }
            let id_str = parts[0];
            let size_str = parts[1];
            let id = u64::from_str_radix(id_str, 16)
                .map_err(|e| format!("invalid temp id '{}': {}", id_str, e))?;
            let size = size_str
                .parse::<usize>()
                .map_err(|e| format!("invalid size '{}': {}", size_str, e))?;
            Ok(Varnode::Temporary { id, size })
        }
        "const" => {
            let s = pair.as_str();
            let parts: Vec<&str> = s.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(format!("const missing size: {}", s));
            }
            let num_str = parts[0];
            let size_str = parts[1];
            let value = if num_str.starts_with("0x") {
                let hex_part = &num_str[2..];
                u64::from_str_radix(hex_part, 16)
                    .map_err(|e| format!("invalid hex const '{}': {}", hex_part, e))?
            } else {
                num_str
                    .parse::<u64>()
                    .map_err(|e| format!("invalid dec const '{}': {}", num_str, e))?
            };
            let size = size_str
                .parse::<usize>()
                .map_err(|e| format!("invalid size '{}': {}", size_str, e))?;
            Ok(Varnode::Const { value, size })
        }
        "register" => {
            let s = pair.as_str();
            let ch = s
                .chars()
                .next()
                .ok_or_else(|| "empty register".to_string())?;
            Ok(Varnode::Register { name: ch })
        }
        "memory" => {
            let s = pair.as_str();
            if let Some(close_idx) = s.find(']') {
                if !s.starts_with('[') || close_idx < 2 {
                    return Err(format!("invalid memory syntax: {}", s));
                }
                let space = s[1..close_idx].to_string();
                let const_part = &s[(close_idx + 1)..];
                let parts: Vec<&str> = const_part.splitn(2, ':').collect();
                if parts.len() != 2 {
                    return Err(format!("memory const missing size: {}", s));
                }
                let num_str = parts[0];
                let size_str = parts[1];
                let value = if num_str.starts_with("0x") {
                    let hex_part = &num_str[2..];
                    u64::from_str_radix(hex_part, 16)
                        .map_err(|e| format!("invalid hex const '{}': {}", hex_part, e))?
                } else {
                    num_str
                        .parse::<u64>()
                        .map_err(|e| format!("invalid dec const '{}': {}", num_str, e))?
                };
                let size = size_str
                    .parse::<usize>()
                    .map_err(|e| format!("invalid size '{}': {}", size_str, e))?;
                Ok(Varnode::Memory { space, value, size })
            } else {
                Err(format!("invalid memory form: {}", s))
            }
        }
        "varnode" => {
            let mut inner = pair.into_inner();
            if let Some(inner_pair) = inner.next() {
                varnode_from_pair(inner_pair)
            } else {
                Err("empty varnode".to_string())
            }
        }
        other => Err(format!("unexpected rule in varnode_from_pair: {}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_temporary() {
        let s = "$U4:8";
        let vn = parse_varnode(s).expect("should parse temporary");
        assert_eq!(vn, Varnode::Temporary { id: 0x4, size: 8 });
    }

    #[test]
    fn test_parse_const_hex() {
        let s = "0xA:4";
        let vn = parse_varnode(s).expect("should parse hex const");
        assert_eq!(
            vn,
            Varnode::Const {
                value: 0xA,
                size: 4
            }
        );
    }

    #[test]
    fn test_parse_const_dec() {
        let s = "5:2";
        let vn = parse_varnode(s).expect("should parse dec const");
        assert_eq!(vn, Varnode::Const { value: 5, size: 2 });
    }

    #[test]
    fn test_parse_register() {
        let s = "r";
        let vn = parse_varnode(s).expect("should parse register");
        assert_eq!(vn, Varnode::Register { name: 'r' });
    }

    #[test]
    fn test_parse_memory() {
        let s = "[RAM]0x1:4";
        let vn = parse_varnode(s).expect("should parse memory");
        assert_eq!(
            vn,
            Varnode::Memory {
                space: "RAM".to_string(),
                value: 0x1,
                size: 4
            }
        );
    }

    #[test]
    fn test_parse_copy_op() {
        // dest register 'a', src temporary
        let s = "a=COPY$U4:8";
        let op = parse_copy(s).expect("should parse COPY");
        assert_eq!(
            op,
            CopyOp {
                dest: Varnode::Register { name: 'a' },
                src: Varnode::Temporary { id: 0x4, size: 8 }
            }
        );
    }

    #[test]
    fn test_parse_copy_with_const() {
        let s = "x=COPY0xA:4";
        let op = parse_copy(s).expect("should parse COPY with const src");
        assert_eq!(
            op,
            CopyOp {
                dest: Varnode::Register { name: 'x' },
                src: Varnode::Const {
                    value: 0xA,
                    size: 4
                }
            }
        );
    }
}
