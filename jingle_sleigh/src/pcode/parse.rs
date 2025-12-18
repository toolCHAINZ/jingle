use pest_derive::Parser;

use crate::{JingleSleighError, PcodeOperation, SleighArchInfo};

#[derive(Parser)]
#[grammar = "pcode/grammar.pest"]
pub struct PcodeParser;

pub fn parse_pcode<T: AsRef<str>>(
    s: T,
    info: SleighArchInfo,
) -> Result<Vec<PcodeOperation>, JingleSleighError> {
    todo!()
}
