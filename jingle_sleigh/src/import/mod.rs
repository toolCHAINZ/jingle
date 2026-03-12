use crate::VarNode;
use crate::context::SleighContext;
use crate::context::{CallInfo, ModelingBehavior, ParameterLocation};
use crate::error::JingleSleighError;
use crate::space::SleighArchInfo;
use serde::Deserialize;
use std::io::Read;
use std::path::Path;

pub struct ImportStats {
    pub functions_loaded: usize,
}

/// Load calling convention metadata for exported functions from a Ghidra XML export
/// (`File → Export Program → XML`).
///
/// For each `<FUNCTION>` entry the importer:
/// - parses `ENTRY_POINT` and `CALLING_CONVENTION` attributes
/// - converts each `<PARAMETER STORAGE="...">` into a `ParameterLocation`
/// - enriches with return varnodes, extrapop, and killed registers from the named convention
/// - registers the resulting `CallInfo` via `ctx.add_call_metadata()`
pub fn load_from_ghidra_xml<P: AsRef<Path>>(
    ctx: &mut SleighContext,
    path: P,
) -> Result<ImportStats, JingleSleighError> {
    let mut file = std::fs::File::open(path.as_ref())
        .map_err(|e| JingleSleighError::IoError(e.to_string()))?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| JingleSleighError::IoError(e.to_string()))?;

    let program: GhidraProgram = serde_xml_rs::from_str(&content)?;

    let arch = ctx.arch_info().clone();
    let cc_info = ctx.calling_convention_info().clone();

    let functions = program
        .functions
        .and_then(|f| f.function)
        .unwrap_or_default();

    let mut stats = ImportStats {
        functions_loaded: 0,
    };

    for func in functions {
        let addr = match parse_hex_or_dec(&func.entry_point) {
            Some(a) => a as u64,
            None => continue,
        };

        let convention_name = func.calling_convention.as_deref();

        // Resolve prototype: named convention, or fall back to default.
        let proto = convention_name
            .and_then(|name| cc_info.call_conventions().iter().find(|p| p.name == name))
            .or_else(|| cc_info.default_calling_convention());

        let extrapop = proto.and_then(|p| p.extrapop);

        let killed_regs: Vec<VarNode> = proto
            .map(|p| {
                p.killed_by_call
                    .iter()
                    .filter_map(|name: &String| arch.register(name.as_str()).cloned())
                    .collect()
            })
            .unwrap_or_default();

        let outputs = cc_info.return_varnodes(&arch, convention_name).ok();

        let args: Vec<ParameterLocation> = func
            .parameters
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| parse_storage(&p.storage, &arch))
            .collect();

        let call_info = CallInfo {
            args,
            outputs,
            model_behavior: ModelingBehavior::default(),
            extrapop,
            killed_regs,
        };

        ctx.add_call_metadata(addr, call_info);
        stats.functions_loaded += 1;
    }

    Ok(stats)
}

/// Parse a Ghidra STORAGE string into a `ParameterLocation`.
///
/// Formats:
/// - `"ECX:4"` → `Register(ECX varnode)` — register size is taken from arch, not the string
/// - `"Stack[0x4]:4"` → `Stack { offset: 4, size: 4 }`
///
/// Returns `None` for unrecognised formats; that parameter is silently skipped.
fn parse_storage(storage: &str, arch: &SleighArchInfo) -> Option<ParameterLocation> {
    if let Some(rest) = storage.strip_prefix("Stack[") {
        let bracket = rest.find(']')?;
        let offset = parse_hex_or_dec(&rest[..bracket])?;
        let size_str = rest.get(bracket + 2..)?;
        let size = size_str.parse::<u32>().ok()?;
        return Some(ParameterLocation::Stack { offset, size });
    }

    let (reg, _size_str) = storage.rsplit_once(':')?;
    let vn = arch.register(reg)?.clone();
    Some(ParameterLocation::Register(vn))
}

fn parse_hex_or_dec(s: &str) -> Option<i64> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<i64>().ok()
    }
}

#[derive(Deserialize)]
#[serde(rename = "PROGRAM")]
struct GhidraProgram {
    #[serde(rename = "FUNCTIONS")]
    functions: Option<GhidraFunctions>,
}

#[derive(Deserialize)]
struct GhidraFunctions {
    #[serde(rename = "FUNCTION")]
    function: Option<Vec<GhidraFunction>>,
}

#[derive(Deserialize)]
struct GhidraFunction {
    #[serde(rename = "@ENTRY_POINT")]
    entry_point: String,
    #[serde(rename = "@CALLING_CONVENTION")]
    calling_convention: Option<String>,
    #[serde(rename = "PARAMETER")]
    parameters: Option<Vec<GhidraParameter>>,
}

#[derive(Deserialize)]
struct GhidraParameter {
    #[serde(rename = "@STORAGE")]
    storage: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::SleighContextBuilder;
    use std::path::Path;

    const X86_32: &str = "x86:LE:32:default";
    const X86_64: &str = "x86:LE:64:default";

    fn build_ctx(arch: &str) -> SleighContext {
        SleighContextBuilder::load_ghidra_installation(Path::new("/Applications/ghidra"))
            .unwrap()
            .build(arch)
            .unwrap()
    }

    #[test]
    fn test_parse_storage_stack() {
        let ctx = build_ctx(X86_32);
        let arch = ctx.arch_info();
        let loc = parse_storage("Stack[0x4]:4", arch).unwrap();
        assert_eq!(loc, ParameterLocation::Stack { offset: 4, size: 4 });
    }

    #[test]
    fn test_parse_storage_stack_decimal_offset() {
        let ctx = build_ctx(X86_32);
        let arch = ctx.arch_info();
        let loc = parse_storage("Stack[8]:4", arch).unwrap();
        assert_eq!(loc, ParameterLocation::Stack { offset: 8, size: 4 });
    }

    #[test]
    fn test_parse_storage_register() {
        let ctx = build_ctx(X86_32);
        let arch = ctx.arch_info();
        let loc = parse_storage("ECX:4", arch).unwrap();
        assert_eq!(
            loc,
            ParameterLocation::Register(arch.register("ECX").unwrap().clone())
        );
    }

    #[test]
    fn test_parse_storage_malformed_returns_none() {
        let ctx = build_ctx(X86_32);
        let arch = ctx.arch_info();
        assert!(parse_storage("NOTAREGISTER", arch).is_none());
        assert!(parse_storage("Stack[]:4", arch).is_none());
    }

    #[test]
    fn test_load_xml_register_args() {
        let mut ctx = build_ctx(X86_64);
        let arch = ctx.arch_info().clone();

        let xml = r#"<?xml version="1.0"?>
<PROGRAM NAME="test" IMAGE_BASE="0x400000">
  <FUNCTIONS>
    <FUNCTION ENTRY_POINT="0x401000" NAME="foo" CALLING_CONVENTION="__stdcall">
      <PARAMETER ORDINAL="0" STORAGE="RDI:8"/>
      <PARAMETER ORDINAL="1" STORAGE="RSI:8"/>
    </FUNCTION>
  </FUNCTIONS>
</PROGRAM>"#;

        let tmp = std::env::temp_dir().join("jingle_test_register_args.xml");
        std::fs::write(&tmp, xml).unwrap();

        let stats = load_from_ghidra_xml(&mut ctx, &tmp).unwrap();
        assert_eq!(stats.functions_loaded, 1);

        // Verify args were stored
        let call_info = ctx.metadata.func_info.get(&0x401000).unwrap();
        assert_eq!(call_info.args.len(), 2);
        assert_eq!(
            call_info.args[0],
            ParameterLocation::Register(arch.register("RDI").unwrap().clone())
        );
        assert_eq!(
            call_info.args[1],
            ParameterLocation::Register(arch.register("RSI").unwrap().clone())
        );
    }

    #[test]
    fn test_load_xml_stack_args() {
        let mut ctx = build_ctx(X86_32);

        let xml = r#"<?xml version="1.0"?>
<PROGRAM NAME="test" IMAGE_BASE="0x400000">
  <FUNCTIONS>
    <FUNCTION ENTRY_POINT="0x402000" NAME="cdecl_fn" CALLING_CONVENTION="__cdecl">
      <PARAMETER ORDINAL="0" STORAGE="Stack[0x4]:4"/>
      <PARAMETER ORDINAL="1" STORAGE="Stack[0x8]:4"/>
    </FUNCTION>
  </FUNCTIONS>
</PROGRAM>"#;

        let tmp = std::env::temp_dir().join("jingle_test_stack_args.xml");
        std::fs::write(&tmp, xml).unwrap();

        let stats = load_from_ghidra_xml(&mut ctx, &tmp).unwrap();
        assert_eq!(stats.functions_loaded, 1);

        let call_info = ctx.metadata.func_info.get(&0x402000).unwrap();
        assert_eq!(call_info.args.len(), 2);
        assert_eq!(
            call_info.args[0],
            ParameterLocation::Stack { offset: 4, size: 4 }
        );
        assert_eq!(
            call_info.args[1],
            ParameterLocation::Stack { offset: 8, size: 4 }
        );
    }

    #[test]
    fn test_load_xml_unknown_convention_falls_back_to_default() {
        let mut ctx = build_ctx(X86_64);

        let xml = r#"<?xml version="1.0"?>
<PROGRAM NAME="test" IMAGE_BASE="0x400000">
  <FUNCTIONS>
    <FUNCTION ENTRY_POINT="0x403000" NAME="unknown_cc" CALLING_CONVENTION="__nonexistent_cc">
      <PARAMETER ORDINAL="0" STORAGE="RDI:8"/>
    </FUNCTION>
  </FUNCTIONS>
</PROGRAM>"#;

        let tmp = std::env::temp_dir().join("jingle_test_unknown_cc.xml");
        std::fs::write(&tmp, xml).unwrap();

        let stats = load_from_ghidra_xml(&mut ctx, &tmp).unwrap();
        assert_eq!(stats.functions_loaded, 1);

        // extrapop should come from the default convention
        let call_info = ctx.metadata.func_info.get(&0x403000).unwrap();
        let default_extrapop = ctx
            .calling_convention_info()
            .default_calling_convention()
            .and_then(|p| p.extrapop);
        assert_eq!(call_info.extrapop, default_extrapop);
    }

    #[test]
    fn test_load_xml_malformed_storage_skipped() {
        let mut ctx = build_ctx(X86_32);

        let xml = r#"<?xml version="1.0"?>
<PROGRAM NAME="test" IMAGE_BASE="0x400000">
  <FUNCTIONS>
    <FUNCTION ENTRY_POINT="0x404000" NAME="partial">
      <PARAMETER ORDINAL="0" STORAGE="NOTAREGISTER"/>
      <PARAMETER ORDINAL="1" STORAGE="Stack[0x4]:4"/>
    </FUNCTION>
  </FUNCTIONS>
</PROGRAM>"#;

        let tmp = std::env::temp_dir().join("jingle_test_malformed_storage.xml");
        std::fs::write(&tmp, xml).unwrap();

        let stats = load_from_ghidra_xml(&mut ctx, &tmp).unwrap();
        assert_eq!(stats.functions_loaded, 1);

        // Malformed first param skipped; second is valid
        let call_info = ctx.metadata.func_info.get(&0x404000).unwrap();
        assert_eq!(call_info.args.len(), 1);
        assert_eq!(
            call_info.args[0],
            ParameterLocation::Stack { offset: 4, size: 4 }
        );
    }
}
