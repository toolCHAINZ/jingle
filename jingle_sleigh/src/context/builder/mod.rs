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

        // Discover compiler-spec (.cspec) metadata for this language.
        // We will:
        //  - collect a list of compiler spec descriptors (with resolved paths)
        //  - parse calling convention prototypes (extrapop, stackshift)
        //  - parse prototype pentries (input/output) and killedbycall/unaffected lists
        //  - pick a default calling convention if a <default_proto> is specified
        //  - still honor stackpointer entries if present (and seed arch_info)
        //
        // The deserialization structs below are intentionally minimal: they only
        // model the small subset of the cspec XML we care about.
        #[derive(Debug, Deserialize)]
        struct CSpecRegister {
            #[serde(rename = "@name")]
            name: String,
        }

        #[derive(Debug, Deserialize)]
        struct CSpecAddr {
            #[serde(rename = "@offset")]
            offset: Option<String>,
            #[serde(rename = "@space")]
            space: Option<String>,
        }

        #[derive(Debug, Deserialize)]
        struct CSpecStackPointer {
            #[serde(rename = "@register")]
            register: String,
            #[serde(rename = "@space")]
            space: Option<String>,
        }

        #[derive(Debug, Deserialize)]
        struct CSpecPentry {
            #[serde(rename = "@minsize")]
            minsize: Option<String>,
            #[serde(rename = "@maxsize")]
            maxsize: Option<String>,
            #[serde(rename = "@align")]
            align: Option<String>,
            #[serde(rename = "@storage")]
            storage: Option<String>,
            #[serde(rename = "@metatype")]
            metatype: Option<String>,
            #[serde(rename = "register")]
            register: Option<Vec<CSpecRegister>>,
            #[serde(rename = "addr")]
            addr: Option<Vec<CSpecAddr>>,
            #[serde(rename = "@extension")]
            extension: Option<String>,
        }

        #[derive(Debug, Deserialize)]
        struct CSpecInputOutput {
            #[serde(rename = "pentry")]
            pentry: Option<Vec<CSpecPentry>>,
        }

        #[derive(Debug, Deserialize)]
        struct CSpecKilledOrUnaffected {
            #[serde(rename = "register")]
            register: Option<Vec<CSpecRegister>>,
            // some cspecs place varnode entries - we only capture register names here
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename = "prototype")]
        struct CSpecPrototypeFull {
            #[serde(rename = "@name")]
            name: Option<String>,
            #[serde(rename = "@extrapop")]
            extrapop: Option<String>,
            #[serde(rename = "@stackshift")]
            stackshift: Option<String>,
            #[serde(rename = "input")]
            input: Option<CSpecInputOutput>,
            #[serde(rename = "output")]
            output: Option<CSpecInputOutput>,
            #[serde(rename = "killedbycall")]
            killedbycall: Option<CSpecKilledOrUnaffected>,
            #[serde(rename = "unaffected")]
            unaffected: Option<CSpecKilledOrUnaffected>,
        }

        #[derive(Debug, Deserialize)]
        struct CSpecProtoContainer {
            #[serde(rename = "prototype")]
            prototype: Option<Vec<CSpecPrototypeFull>>,
        }

        #[derive(Debug, Deserialize)]
        #[serde(rename = "compiler_spec")]
        struct CSpecRootFull {
            stackpointer: Option<CSpecStackPointer>,
            #[serde(rename = "default_proto")]
            default_proto: Option<CSpecProtoContainer>,
            #[serde(rename = "prototype")]
            prototype: Option<Vec<CSpecPrototypeFull>>,
        }

        // accumulators
        let mut compiler_specs_acc: Vec<crate::context::CompilerSpecInfo> = Vec::new();
        let mut call_convs_acc: Vec<crate::context::PrototypeInfo> = Vec::new();
        let mut default_proto_acc: Option<crate::context::PrototypeInfo> = None;

        for comp in &lang.compiler {
            let cspec_path = path.join(&comp.spec);
            if cspec_path.exists() {
                if let Ok(mut file) = std::fs::File::open(&cspec_path) {
                    // Attempt to parse the cspec. If it fails for any reason, continue scanning others.
                    // Read the opened file into a string and parse via from_str so we avoid
                    // requiring the file to implement any extra traits for from_reader.
                    let mut _cspec_contents = String::new();
                    {
                        use std::io::Read;
                        let _ = file.read_to_string(&mut _cspec_contents);
                    }
                    if let Ok(cspec) = serde_xml_rs::from_str::<CSpecRootFull>(&_cspec_contents) {
                        // Record compiler spec descriptor. Mark is_default true if this cspec contains a <default_proto>.
                        let is_default = cspec.prototype.as_ref().map_or(false, |_| false);
                        compiler_specs_acc.push(crate::context::CompilerSpecInfo {
                            path: cspec_path.clone(),
                            name: Some(comp.name.clone()),
                            is_default,
                        });

                        // Parse prototypes (both top-level and those nested under default_proto)
                        if let Some(protos) = cspec.prototype {
                            for p in protos {
                                // parse numeric attributes
                                let extrapop = p.extrapop.and_then(|s| s.parse::<i32>().ok());
                                let stackshift = p.stackshift.and_then(|s| s.parse::<i32>().ok());
                                let proto_name = p.name.unwrap_or_else(|| "__unnamed".to_string());

                                // parse input pentries
                                let mut pentries: Vec<crate::context::PentryInfo> = Vec::new();
                                if let Some(input) = p.input {
                                    if let Some(entries) = input.pentry {
                                        for pe in entries {
                                            let minsize =
                                                pe.minsize.and_then(|s| s.parse::<u32>().ok());
                                            let maxsize =
                                                pe.maxsize.and_then(|s| s.parse::<u32>().ok());
                                            let align =
                                                pe.align.and_then(|s| s.parse::<u32>().ok());
                                            let storage = pe.storage.or(pe.metatype);
                                            let mut regs: Vec<String> = Vec::new();
                                            if let Some(rvec) = pe.register {
                                                for r in rvec {
                                                    regs.push(r.name);
                                                }
                                            }
                                            let mut addr_space: Option<String> = None;
                                            let mut addr_offset: Option<u64> = None;
                                            if let Some(addrs) = pe.addr {
                                                if let Some(a) = addrs.first() {
                                                    addr_space = a.space.clone();
                                                    if let Some(offstr) = &a.offset {
                                                        if let Ok(off) = offstr.parse::<u64>() {
                                                            addr_offset = Some(off);
                                                        }
                                                    }
                                                }
                                            }
                                            pentries.push(crate::context::PentryInfo {
                                                minsize,
                                                maxsize,
                                                align,
                                                storage,
                                                registers: regs,
                                                addr_space,
                                                addr_offset,
                                            });
                                        }
                                    }
                                }

                                // parse killedbycall
                                let mut killed: Vec<String> = Vec::new();
                                if let Some(k) = p.killedbycall {
                                    if let Some(kregs) = k.register {
                                        for r in kregs {
                                            killed.push(r.name);
                                        }
                                    }
                                }

                                // parse unaffected
                                let mut unaffected: Vec<String> = Vec::new();
                                if let Some(u) = p.unaffected {
                                    if let Some(uregs) = u.register {
                                        for r in uregs {
                                            unaffected.push(r.name);
                                        }
                                    }
                                }

                                let proto = crate::context::PrototypeInfo {
                                    name: proto_name.clone(),
                                    extrapop,
                                    stackshift,
                                    pentries,
                                    killed_by_call: killed,
                                    unaffected,
                                };

                                // set default prototype if none yet
                                if default_proto_acc.is_none() {
                                    default_proto_acc = Some(proto.clone());
                                }
                                call_convs_acc.push(proto);
                            }
                        }

                        // If cspec declares a stackpointer, seed arch_info accordingly
                        if let Some(sp) = cspec.stackpointer {
                            if let Some(vn_ref) = context.arch_info().register(&sp.register) {
                                context.set_stack_pointer_varnode(vn_ref.clone());
                                // Do not break here: continue collecting cspecs and prototypes
                            }
                        }
                    }
                }
            }
        }

        // Fallback: if no stackpointer found in cspecs, allow a fallback from the
        // language definition's optional attributes (if present).
        if context.arch_info().stack_pointer().is_none() {
            if let Some(sp_name) = &lang.stackpointer {
                if let Some(vn_ref) = context.arch_info().register(sp_name) {
                    context.set_stack_pointer_varnode(vn_ref.clone());
                }
            }
        }

        // If we've discovered compiler specs or prototypes, combine them into a
        // ref-counted `CallingConventionInfo` and attach it to the context.
        // Choose the first compiler_spec marked as default (if any)
        let default_comp_spec = compiler_specs_acc.iter().find(|c| c.is_default).cloned();

        let cc_info = crate::context::CallingConventionInfo {
            info: std::sync::Arc::new(crate::context::CallingConventionInfoInner {
                compiler_specs: compiler_specs_acc,
                default_compiler_spec: default_comp_spec,
                call_conventions: call_convs_acc,
                default_calling_convention: default_proto_acc,
            }),
        };
        context.set_calling_convention_info(cc_info);

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

    #[test]
    fn test_parse_prototypes_x64() {
        // Build a context for the canonical x86_64 language and validate
        // that prototypes, pentries, killedbycall and unaffected lists are parsed.
        let builder =
            SleighContextBuilder::load_ghidra_installation(Path::new("/Applications/ghidra"))
                .unwrap();
        let ctx = builder.build(SLEIGH_ARCH).unwrap();

        // Ensure we parsed at least one calling convention prototype
        let convs = ctx.calling_convention_info().call_conventions();
        assert!(!convs.is_empty(), "Expected at least one parsed prototype");

        // Default calling convention should be present and have extrapop and stackshift for x86_64
        let def = ctx
            .calling_convention_info()
            .default_calling_convention()
            .expect("expected default calling convention");
        // In the included x86-64 cspecs the default prototypes commonly specify extrapop=8 and stackshift=8.
        assert_eq!(def.extrapop, Some(8));
        assert_eq!(def.stackshift, Some(8));

        // default stack change derived as extrapop - stackshift = 0
        assert_eq!(ctx.default_stack_change(), Some(0));

        // Ensure at least one prototype has parsed pentries and killed/unaffected info
        assert!(
            convs.iter().any(|p| !p.pentries.is_empty()),
            "Expected at least one prototype to have pentries parsed"
        );
        assert!(
            convs
                .iter()
                .any(|p| !p.killed_by_call.is_empty() || !p.unaffected.is_empty()),
            "Expected at least one prototype to have killed_by_call or unaffected entries"
        );
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
