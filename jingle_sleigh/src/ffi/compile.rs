use crate::ffi::compile::bridge::{CompileDefine, CompileParams};
use std::collections::BTreeMap;
use std::path::Path;

pub struct SleighCompileParams {
    defines: BTreeMap<String, String>,
    unnecessary_pcode_warning: bool,
    lenient_conflict: bool,
    all_collision_warning: bool,
    all_nop_warning: bool,
    dead_temp_warning: bool,
    enforce_local_keyword: bool,
    large_temporary_warning: bool,
    case_sensitive_register_names: bool,
}

pub fn compile(
    in_path: impl AsRef<Path>,
    out_path: impl AsRef<Path>,
    params: Option<SleighCompileParams>,
) {
    let _hi = Path::new("hi");
    if let Some(in_path) = in_path.as_ref().to_str() {
        if let Some(out_path) = out_path.as_ref().to_str() {
            bridge::compile(in_path, out_path, params.unwrap_or_default().into())
        }
    }
}

impl Default for SleighCompileParams {
    fn default() -> Self {
        Self {
            defines: BTreeMap::new(),
            unnecessary_pcode_warning: false,
            lenient_conflict: true,
            all_collision_warning: false,
            all_nop_warning: false,
            dead_temp_warning: false,
            enforce_local_keyword: false,
            large_temporary_warning: false,
            case_sensitive_register_names: false,
        }
    }
}

impl From<SleighCompileParams> for CompileParams {
    fn from(value: SleighCompileParams) -> Self {
        Self {
            defines: value
                .defines
                .iter()
                .map(|(name, val)| CompileDefine {
                    name: name.clone(),
                    value: val.clone(),
                })
                .collect(),
            unnecessary_pcode_warning: value.unnecessary_pcode_warning,
            lenient_conflict: value.lenient_conflict,
            all_collision_warning: value.all_collision_warning,
            all_nop_warning: value.all_nop_warning,
            dead_temp_warning: value.dead_temp_warning,
            enforce_local_keyword: value.enforce_local_keyword,
            large_temporary_warning: value.large_temporary_warning,
            case_sensitive_register_names: value.case_sensitive_register_names,
        }
    }
}

#[cxx::bridge]
mod bridge {
    struct CompileDefine {
        name: String,
        value: String,
    }

    struct CompileParams {
        defines: Vec<CompileDefine>,
        unnecessary_pcode_warning: bool,
        lenient_conflict: bool,
        all_collision_warning: bool,
        all_nop_warning: bool,
        dead_temp_warning: bool,
        enforce_local_keyword: bool,
        large_temporary_warning: bool,
        case_sensitive_register_names: bool,
    }

    unsafe extern "C++" {
        include!("jingle_sleigh/src/ffi/cpp/compile.h");

        fn compile(inFile: &str, outFile: &str, params: CompileParams);

    }
}
