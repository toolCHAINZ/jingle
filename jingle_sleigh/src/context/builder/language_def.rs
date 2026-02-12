use crate::error::JingleSleighError;
use crate::error::JingleSleighError::LanguageSpecRead;
use serde::Deserialize;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize)]
pub enum SleighEndian {
    #[serde(rename = "little")]
    Little,
    #[serde(rename = "big")]
    Big,
}

#[expect(unused)]
#[derive(Clone, Debug, Deserialize)]
pub struct Compiler {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@spec")]
    pub spec: String,
    #[serde(rename = "@id")]
    pub id: String,
}

#[expect(unused)]
#[derive(Clone, Debug, Deserialize)]
pub struct ExternalName {
    #[serde(rename = "@tool")]
    pub tool: String,
    #[serde(rename = "@name")]
    pub name: String,
}

#[expect(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename = "language")]
pub struct Language {
    #[serde(rename = "@processor")]
    pub processor: String,
    #[serde(rename = "@endian")]
    pub endian: SleighEndian,
    #[serde(rename = "@size")]
    pub size: String,
    #[serde(rename = "@variant")]
    pub variant: String,
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "@slafile")]
    pub sla_file: PathBuf,
    #[serde(rename = "@processorspec")]
    pub processor_spec: PathBuf,
    #[serde(rename = "@manualindexfile")]
    pub manual_index_file: Option<PathBuf>,
    #[serde(rename = "@id")]
    pub id: String,
    /// Optional attribute to capture a language's declared stack pointer.
    /// This is intended to represent a reference (e.g., register name) that may
    /// be present in the language metadata and which will be used later to seed
    /// the SleighArchInfo with an appropriate varnode.
    #[serde(rename = "@stackpointer")]
    pub stackpointer: Option<String>,
    /// Optional attribute to capture a language's declared program counter.
    /// This typically points to the register name used as the PC and will be
    /// parsed from processor specs and represented in the arch info.
    #[serde(rename = "@programcounter")]
    pub programcounter: Option<String>,
    pub description: String,
    pub compiler: Vec<Compiler>,
    pub external_name: Option<Vec<ExternalName>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "language_definitions")]
struct LanguageDefinitions {
    #[serde(rename = "#content")]
    pub language_definitions: Vec<Language>,
}

pub(super) fn parse_ldef(path: &Path) -> Result<Vec<Language>, JingleSleighError> {
    let file = File::open(path).map_err(|_| LanguageSpecRead)?;
    let def: LanguageDefinitions = serde_xml_rs::from_reader(file)?;
    Ok(def.language_definitions)
}

#[cfg(test)]
mod tests {
    use crate::context::builder::language_def::LanguageDefinitions;
    use serde_xml_rs::from_str;
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn test() {
        let mut file = File::open("ghidra/Ghidra/Processors/x86/data/languages/x86.ldefs").unwrap();
        let mut data: String = String::new();
        file.read_to_string(&mut data).unwrap();
        let _ldef: LanguageDefinitions = from_str(&data).unwrap();
    }
}
