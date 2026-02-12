use crate::context::SleighContext;
use crate::context::builder::language_def::{Language, parse_ldef};
use crate::context::builder::processor_spec::parse_pspec;
use crate::error::JingleSleighError;
use crate::error::JingleSleighError::{InvalidLanguageId, LanguageSpecRead};
use serde::Deserialize;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{Level, event, instrument};

pub(crate) mod language_def;
pub(crate) mod processor_spec;

#[derive(Debug, Default, Clone)]
pub struct SleighContextBuilder {
    defs: Vec<(Language, PathBuf)>,
}

impl SleighContextBuilder {
    pub fn get_language_ids(&self) -> Vec<&str> {
        self.defs.iter().map(|(l, _)| l.id.as_str()).collect()
    }

    fn get_language(&self, id: &str) -> Option<&(Language, PathBuf)> {
        self.defs.iter().find(|(p, _)| p.id.eq(id))
    }
    #[instrument(skip_all, fields(%id))]
    pub fn build(&self, id: &str) -> Result<SleighContext, JingleSleighError> {
        let (lang, path) = self
            .get_language(id)
            .ok_or(InvalidLanguageId(id.to_string()))?;
        let mut context = SleighContext::new(lang, path)?;
        event!(Level::INFO, "Created sleigh context");

        // Parse the processor spec (.pspec) for initial context values and
        // optional <programcounter register="..."/> declaration.
        let pspec_path = path.join(&lang.processor_spec);
        let pspec = parse_pspec(&pspec_path)?;
        // If the pspec declares a program counter register, map it to a varnode
        // (if that register exists in the Sleigh context) and store it in the
        // SleighContext arch info.
        if let Some(pc) = pspec.programcounter {
            if let Some(vn_ref) = context.arch_info().register(&pc.register) {
                context.set_program_counter_varnode(vn_ref.clone());
            }
        }

        // Apply initial context sets (same as before)
        if let Some(ctx_sets) = pspec.context_data.and_then(|d| d.context_set) {
            for set in ctx_sets.sets {
                // todo: gross hack
                if set.value.starts_with("0x") {
                    context.set_initial_context(
                        &set.name,
                        u32::from_str_radix(&set.value[2..], 16).unwrap(),
                    )?;
                } else {
                    context.set_initial_context(&set.name, set.value.parse::<u32>().unwrap())?;
                }
            }
        }

        // Try to discover a stackpointer declared in compiler specs (.cspec).
        // Iterate compilers listed in the language definition and look for a
        // <stackpointer register="..." [space="..."]/> element.
        #[derive(Debug, Deserialize)]
        struct CSpecStackPointer {
            #[serde(rename = "@register")]
            register: String,
            #[serde(rename = "@space")]
            space: Option<String>,
        }
        #[derive(Debug, Deserialize)]
        #[serde(rename = "compiler_spec")]
        struct CSpecRoot {
            stackpointer: Option<CSpecStackPointer>,
        }

        for comp in &lang.compiler {
            let cspec_path = path.join(&comp.spec);
            if cspec_path.exists() {
                if let Ok(file) = std::fs::File::open(&cspec_path) {
                    // `from_reader` returns a Result; bind it first then pattern-match.
                    let cspec_res: Result<CSpecRoot, _> = serde_xml_rs::from_reader(file);
                    if let Ok(cspec) = cspec_res {
                        if let Some(sp) = cspec.stackpointer {
                            if let Some(vn_ref) = context.arch_info().register(&sp.register) {
                                context.set_stack_pointer_varnode(vn_ref.clone());
                                // Found a stack pointer; stop searching further cspecs.
                                break;
                            }
                        }
                    }
                }
            }
        }

        // If no stackpointer was found in cspecs, allow a fallback from the
        // language definition's optional attributes (if present).
        if context.arch_info().register("sp").is_none() {
            if let Some(sp_name) = &lang.stackpointer {
                if let Some(vn_ref) = context.arch_info().register(sp_name) {
                    context.set_stack_pointer_varnode(vn_ref.clone());
                }
            }
        }

        Ok(context)
    }
    pub fn load_folder<T: AsRef<Path>>(path: T) -> Result<Self, JingleSleighError> {
        let ldef = SleighContextBuilder::_load_folder(path.as_ref())?;
        Ok(SleighContextBuilder { defs: ldef })
    }

    fn _load_folder(path: &Path) -> Result<Vec<(Language, PathBuf)>, JingleSleighError> {
        let path = path.canonicalize();
        let path = path.map_err(|_| LanguageSpecRead)?;
        if !path.is_dir() {
            return Err(LanguageSpecRead);
        }
        let ldef_paths = find_ldef(&path)?;
        let defs: Vec<(Language, PathBuf)> = ldef_paths
            .iter()
            .flat_map(|ldef_path| {
                let defs: Vec<Language> = parse_ldef(ldef_path.as_path()).unwrap();
                defs.iter()
                    .map(|f| (f.clone(), path.to_path_buf()))
                    .collect::<Vec<(Language, PathBuf)>>()
            })
            .collect();
        Ok(defs)
    }

    #[instrument]
    pub fn load_ghidra_installation<T: AsRef<Path> + Debug>(
        path: T,
    ) -> Result<Self, JingleSleighError> {
        let path = path.as_ref().join("Ghidra").join("Processors");
        let mut defs = vec![];
        for entry in (path.read_dir().map_err(|_| LanguageSpecRead)?).flatten() {
            let lang_path = entry.path().join("data").join("languages");
            if lang_path.exists() {
                let d = Self::_load_folder(&lang_path)?;
                defs.extend(d);
            }
        }
        Ok(SleighContextBuilder { defs })
    }
}

fn find_ldef(path: &Path) -> Result<Vec<PathBuf>, JingleSleighError> {
    let mut ldefs = vec![];
    for entry in (fs::read_dir(path).map_err(|_| LanguageSpecRead)?).flatten() {
        if let Some(e) = entry.path().extension() {
            if e == "ldefs" {
                ldefs.push(entry.path().clone());
            }
        }
    }
    if ldefs.is_empty() {
        return Err(LanguageSpecRead);
    }
    Ok(ldefs)
}

#[cfg(test)]
mod tests {
    use crate::context::builder::processor_spec::parse_pspec;
    use crate::context::builder::{SleighContextBuilder, parse_ldef};

    use crate::tests::SLEIGH_ARCH;
    use std::path::Path;

    #[test]
    fn test_parse_ldef() {
        parse_ldef(Path::new(
            "ghidra/Ghidra/Processors/x86/data/languages/x86.ldefs",
        ))
        .unwrap();
    }

    #[test]
    fn test_parse_pspec() {
        parse_pspec(Path::new(
            "ghidra/Ghidra/Processors/x86/data/languages/x86.pspec",
        ))
        .unwrap();
    }

    #[test]
    fn test_parse_language_folder() {
        SleighContextBuilder::load_folder(Path::new(
            "ghidra/Ghidra/Processors/x86/data/languages/",
        ))
        .unwrap();
        SleighContextBuilder::load_folder(Path::new("ghidra/Ghidra/Processors/x86/data/languages"))
            .unwrap();
    }

    #[test]
    fn test_parse_language_ghidra() {
        let _builder = SleighContextBuilder::load_ghidra_installation(Path::new("ghidra")).unwrap();
    }

    #[test]
    fn test_get_language() {
        let langs = SleighContextBuilder::load_folder(Path::new(
            "ghidra/Ghidra/Processors/x86/data/languages/",
        ))
        .unwrap();
        assert!(langs.get_language("sdf").is_none());
        assert!(langs.get_language(SLEIGH_ARCH).is_some());
    }

    // New tests to verify stackpointer and programcounter detection logic.
    // These tests attempt to build contexts for languages in the local Ghidra
    // installation and assert that at least one language exposes a detected
    // stack pointer and program counter varnode. The tests are permissive in
    // which language they pick (they iterate languages) to avoid brittle
    // expectations about a single specific language id.
    #[test]
    fn test_stackpointer_detection() {
        let builder =
            SleighContextBuilder::load_ghidra_installation(Path::new("/Applications/ghidra"))
                .unwrap();
        let mut found = false;
        for id in builder.get_language_ids() {
            // Try building the context; some builds may fail for some languages, ignore those.
            if let Ok(ctx) = builder.build(id) {
                if ctx.arch_info().stack_pointer().is_some() {
                    found = true;
                    break;
                }
            }
        }
        assert!(
            found,
            "No language with detected stack pointer found in local Ghidra installation"
        );
    }

    #[test]
    fn test_programcounter_detection() {
        let builder =
            SleighContextBuilder::load_ghidra_installation(Path::new("/Applications/ghidra"))
                .unwrap();
        let mut found = false;
        for id in builder.get_language_ids() {
            if let Ok(ctx) = builder.build(id) {
                if ctx.arch_info().program_counter().is_some() {
                    found = true;
                    break;
                }
            }
        }
        assert!(
            found,
            "No language with detected program counter found in local Ghidra installation"
        );
    }

    #[test]
    fn test_x64_stack_and_pc() {
        // Explicit test against the canonical x86_64 language used elsewhere in the tests.
        let builder =
            SleighContextBuilder::load_ghidra_installation(Path::new("/Applications/ghidra"))
                .unwrap();
        let ctx = builder.build(crate::tests::SLEIGH_ARCH).unwrap();

        // For x86_64 we expect a stack pointer and program counter to be detected.
        let sp = ctx.arch_info().stack_pointer();
        let pc = ctx.arch_info().program_counter();

        assert!(sp.is_some(), "Expected stack pointer varnode for x86_64");
        assert!(pc.is_some(), "Expected program counter varnode for x86_64");

        let sp = sp.unwrap();
        let pc = pc.unwrap();

        // Sanity-check: sizes should be > 0; x86_64 registers will typically be 8 bytes.
        assert!(sp.size > 0);
        assert!(pc.size > 0);
    }
}
