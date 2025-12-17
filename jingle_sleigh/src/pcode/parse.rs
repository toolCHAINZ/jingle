use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "pcode/grammar.pest"]
pub struct PcodeParser;
