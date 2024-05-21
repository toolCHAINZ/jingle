#include "rust/cxx.h"
#include "sleigh/slgh_compile.hh"
#include "jingle_sleigh/src/ffi/compile.rs.h"

void compile(rust::Str infile, rust::Str outFile, CompileParams params) {
    std::string in = infile.operator std::string();
    std::string out = outFile.operator std::string();
    ghidra::SleighCompile compiler;
    std::map<std::string, std::string> defines;
    for (const auto &item: params.defines) {
        std::string name = item.name.operator std::string();
        std::string value = item.name.operator std::string();
        defines[name] = value;
    }
    compiler.setAllOptions(defines, params.unnecessary_pcode_warning, params.lenient_conflict,
                           params.all_collision_warning, params.all_nop_warning, params.dead_temp_warning,
                           params.enforce_local_keyword, params.large_temporary_warning,
                           params.case_sensitive_register_names);
    compiler.run_compilation(in, out);
}
