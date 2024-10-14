use crate::error::JingleSleighError;
use crate::error::JingleSleighError::LanguageSpecRead;
use serde::Deserialize;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(rename = "context_set")]
pub struct ContextSet {
    pub name: String,
    #[serde(rename = "val")]
    pub value: String,
}
#[allow(unused)]
#[derive(Debug, Deserialize)]
#[serde(rename = "context_set")]
pub struct ContextSetSpace {
    pub space: String,
    #[serde(rename = "$value")]
    pub sets: Vec<ContextSet>,
}

#[derive(Debug, Deserialize)]
pub struct ContextData {
    pub context_set: Option<ContextSetSpace>,
    #[allow(unused)]
    pub tracked_set: Option<ContextSetSpace>,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "processor_spec")]
pub struct ProcessorSpec {
    // TODO: Properties
    // properties: Properties
    pub context_data: Option<ContextData>,
}

pub(super) fn parse_pspec(path: &Path) -> Result<ProcessorSpec, JingleSleighError> {
    let file = File::open(path).map_err(|_| LanguageSpecRead)?;
    let def: ProcessorSpec = serde_xml_rs::from_reader(file)?;
    Ok(def)
}

#[cfg(test)]
mod tests {
    use crate::context::builder::processor_spec::ProcessorSpec;
    use serde_xml_rs::from_str;
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn test() {
        let mut file = File::open("ghidra/Ghidra/Processors/x86/data/languages/x86.pspec").unwrap();
        let mut data: String = String::new();
        file.read_to_string(&mut data).unwrap();
        let _pspec: ProcessorSpec = from_str(&data).unwrap();
    }
}
