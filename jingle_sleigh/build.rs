use std::fs;
use std::fs::copy;
use std::path::{Path, PathBuf};

const SLEIGH_SOURCES: &[&str] = &[
    "address.cc",
    "compression.cc",
    "context.cc",
    "globalcontext.cc",
    "float.cc",
    "marshal.cc",
    "opcodes.cc",
    "pcoderaw.cc",
    "semantics.cc",
    "slaformat.cc",
    "sleigh.cc",
    "sleighbase.cc",
    "slghpatexpress.cc",
    "slghpattern.cc",
    "slghsymbol.cc",
    "space.cc",
    "translate.cc",
    "xml.cc",
    "filemanage.cc",
    "pcodecompile.cc",
];

const SLEIGH_HEADERS: &[&str] = &[
    "address.hh",
    "compression.hh",
    "context.hh",
    "error.hh",
    "filemanage.hh",
    "float.hh",
    "globalcontext.hh",
    "loadimage.hh",
    "marshal.hh",
    "opbehavior.hh",
    "opcodes.hh",
    "partmap.hh",
    "pcodecompile.hh",
    "pcoderaw.hh",
    "semantics.hh",
    "slaformat.hh",
    "sleigh.hh",
    "sleighbase.hh",
    "slghpatexpress.hh",
    "slghpattern.hh",
    "slghsymbol.hh",
    "space.hh",
    "translate.hh",
    "types.h",
    "xml.hh",
];

const ZLIB_HEADERS: &[&str] = &[
    "deflate.h",
    "gzguts.h",
    "inffast.h",
    "inffixed.h",
    "inflate.h",
    "inftrees.h",
    "zconf.h",
    "zlib.h",
    "zutil.h",
];

const ZLIB_SOURCES: &[&str] = &[
    "deflate.c",
    "inflate.c",
    "zutil.c",
    "inftrees.c",
    "inffast.c",
    "adler32.c",
];

const JINGLE_CPP_SOURCES: &[&str] = &[
    "context.cpp",
    "dummy_load_image.cpp",
    "rust_load_image.cpp",
    "addrspace_handle.cpp",
    "addrspace_manager_handle.cpp",
    "varnode_translation.cpp",
    "jingle_pcode_emitter.cpp",
    "jingle_assembly_emitter.cpp",
];

const RUST_FFI_BRIDGES: &[&str] = &[
    "addrspace.rs",
    "context_ffi.rs",
    "instruction.rs",
    "opcode.rs",
];

fn main() {
    if cfg!(target_os = "macos") {
        println!("cargo::rustc-link-search=/opt/homebrew/lib")
    }
    if !sleigh_path().exists() | !zlib_path().exists() {
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

    let map_path = |p: fn() -> PathBuf| {
        move |s: &&str| {
            let mut b = p();
            b.push(s);
            b
        }
    };

    let rust_bridges: Vec<PathBuf> = RUST_FFI_BRIDGES.iter().map(map_path(ffi_rs_path)).collect();

    let jingle_cpp_sources: Vec<PathBuf> = JINGLE_CPP_SOURCES
        .iter()
        .map(map_path(ffi_cpp_path))
        .collect();

    let sleigh_sources: Vec<PathBuf> = SLEIGH_SOURCES.iter().map(map_path(sleigh_path)).collect();
    let zlib_sources: Vec<PathBuf> = ZLIB_SOURCES.iter().map(map_path(zlib_path)).collect();

    // This assumes all your C++ bindings are in lib
    let mut bridge = cxx_build::bridges(&rust_bridges);
    bridge
        .files(jingle_cpp_sources)
        .files(sleigh_sources)
        .files(zlib_sources)
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-DLOCAL_ZLIB")
        .flag_if_supported("-DNO_GZIP")
        .flag_if_supported("-Wno-register")
        .flag_if_supported("-w");

    if cfg!(windows) {
        bridge.flag_if_supported("-D_WINDOWS");
    }
    bridge.compile("jingle_sleigh");

    println!("cargo::rerun-if-changed=src/ffi/cpp/");
    for src in rust_bridges {
        println!("cargo::rerun-if-changed={}", src.to_str().unwrap());
    }
}

fn copy_sources() {
    copy_cpp_sources(
        ghidra_sleigh_path(),
        sleigh_path(),
        SLEIGH_SOURCES,
        SLEIGH_HEADERS,
    );
    copy_cpp_sources(ghidra_zlib_path(), zlib_path(), ZLIB_SOURCES, ZLIB_HEADERS);
}

fn copy_cpp_sources<T: AsRef<Path>, E: AsRef<Path>>(
    inpath: T,
    outpath: E,
    sources: &[&str],
    headers: &[&str],
) {
    let _ = fs::create_dir(&outpath);
    for direntry in fs::read_dir(inpath).unwrap().flatten() {
        let path = direntry.path();
        let filename = path.file_name();
        if let Some(filename) = filename {
            let filename = filename.to_str().unwrap();
            if sources.contains(&filename) || headers.contains(&filename) {
                let mut result = PathBuf::from(outpath.as_ref());
                result.push(filename);
                copy(direntry.path(), result.as_path()).unwrap();
                println!(
                    "Copying {} ({} => {})",
                    filename,
                    direntry.path().to_str().unwrap(),
                    result.to_str().unwrap()
                );
            }
        }
    }
}

fn ffi_rs_path() -> PathBuf {
    let mut p = PathBuf::new();
    p.push("src");
    p.push("ffi");
    p
}

fn ffi_cpp_path() -> PathBuf {
    let mut p = ffi_rs_path();
    p.push("cpp");
    p
}

fn sleigh_path() -> PathBuf {
    let mut p = ffi_cpp_path();
    p.push("sleigh");
    p
}

fn zlib_path() -> PathBuf {
    let mut p = ffi_cpp_path();
    p.push("zlib");
    p
}

fn submod_path() -> PathBuf {
    let mut p = PathBuf::new();
    p.push("ghidra");
    p
}

fn ghidra_sleigh_path() -> PathBuf {
    let mut p = submod_path();
    p.push("Ghidra");
    p.push("Features");
    p.push("Decompiler");
    p.push("src");
    p.push("decompile");
    p.push("cpp");
    p
}

fn ghidra_zlib_path() -> PathBuf {
    let mut p = submod_path();
    p.push("Ghidra");
    p.push("Features");
    p.push("Decompiler");
    p.push("src");
    p.push("decompile");
    p.push("zlib");
    p
}
