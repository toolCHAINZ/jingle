use std::fs;
use std::fs::copy;
use std::path::PathBuf;
fn main() {
    if cfg!(target_os = "macos") {
        println!("cargo::rustc-link-search=/opt/homebrew/lib")
    }
    if !cpp_src_path().exists() {
        let submod = submod_path();
        if !submod.read_dir().is_ok_and(|f| f.count() != 0) {
            panic!(
                "SLEIGH sources not found! This likely means that you are developing on a fresh \
            clone of jingle and need to pull in the SLEIGH sources. Please run: \n\
            git submodule init && git submodule update"
            )
        }
        copy_sources();
    }

    let rust_sources = vec![
        "src/ffi/addrspace.rs",
        "src/ffi/context_ffi.rs",
        "src/ffi/instruction.rs",
        "src/ffi/opcode.rs",
    ];

    let cpp_sources = vec![
        "src/ffi/cpp/sleigh/address.cc",
        "src/ffi/cpp/sleigh/compression.cc",
        "src/ffi/cpp/sleigh/context.cc",
        "src/ffi/cpp/sleigh/globalcontext.cc",
        "src/ffi/cpp/sleigh/float.cc",
        "src/ffi/cpp/sleigh/marshal.cc",
        "src/ffi/cpp/sleigh/opcodes.cc",
        "src/ffi/cpp/sleigh/pcoderaw.cc",
        "src/ffi/cpp/sleigh/semantics.cc",
        "src/ffi/cpp/sleigh/slaformat.cc",
        "src/ffi/cpp/sleigh/sleigh.cc",
        "src/ffi/cpp/sleigh/sleighbase.cc",
        "src/ffi/cpp/sleigh/slghpatexpress.cc",
        "src/ffi/cpp/sleigh/slghpattern.cc",
        "src/ffi/cpp/sleigh/slghsymbol.cc",
        "src/ffi/cpp/sleigh/space.cc",
        "src/ffi/cpp/sleigh/translate.cc",
        "src/ffi/cpp/sleigh/xml.cc",
        "src/ffi/cpp/sleigh/filemanage.cc",
        "src/ffi/cpp/sleigh/pcodecompile.cc",
        "src/ffi/cpp/sleigh/slghscan.cc",
        "src/ffi/cpp/sleigh/slghparse.cc",
        "src/ffi/cpp/context.cpp",
        "src/ffi/cpp/dummy_load_image.cpp",
        "src/ffi/cpp/rust_load_image.cpp",
        "src/ffi/cpp/addrspace_handle.cpp",
        "src/ffi/cpp/addrspace_manager_handle.cpp",
        "src/ffi/cpp/varnode_translation.cpp",
        "src/ffi/cpp/jingle_pcode_emitter.cpp",
        "src/ffi/cpp/jingle_assembly_emitter.cpp",
    ];
    // This assumes all your C++ bindings are in lib
    cxx_build::bridges(&rust_sources)
        .files(cpp_sources)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-Dmain=c_main")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-Wno-unneeded-internal-declaration")
        .flag_if_supported("-Wno-format")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-sign-compare")
        .flag_if_supported("-Wno-deprecated-copy-with-user-provided-copy")
        .compile("jingle_sleigh");

    println!("cargo::rerun-if-changed=src/ffi/cpp/");
    for src in rust_sources {
        println!("cargo::rerun-if-changed={src}");
    }
    println!("cargo::rerun-if-changed=src/ffi/addrspace.rs");
    println!("cargo::rerun-if-changed=src/ffi/context_ffi.rs");
    println!("cargo::rerun-if-changed=src/ffi/instruction.rs");
    println!(
        "cargo::rerun-if-changed={}",
        ghidra_cpp_path().to_str().unwrap()
    );
}

fn copy_sources() {
    fs::create_dir(cpp_src_path()).unwrap();
    for path in fs::read_dir(ghidra_cpp_path()).unwrap().flatten() {
        if let Some(name) = path.file_name().to_str() {
            if name.ends_with(".cc") || name.ends_with(".hh") || name.ends_with(".h") {
                let mut result = cpp_src_path();
                result.push(name);
                copy(path.path().as_path(), result.as_path()).unwrap();
                println!("Copying {}", name)
            }
        }
    }
}

fn cpp_src_path() -> PathBuf {
    let mut p = PathBuf::new();
    p.push("src");
    p.push("ffi");
    p.push("cpp");
    p.push("sleigh");
    p
}

fn submod_path() -> PathBuf {
    let mut p = PathBuf::new();
    p.push("ghidra");
    p
}

fn ghidra_cpp_path() -> PathBuf {
    let mut p = PathBuf::new();
    p.push(submod_path());
    p.push("Ghidra");
    p.push("Features");
    p.push("Decompiler");
    p.push("src");
    p.push("decompile");
    p.push("cpp");
    p
}
