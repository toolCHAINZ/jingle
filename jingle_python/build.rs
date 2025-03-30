
fn main() {
    let var = std::env::var("Z3_PYTHON_LIB").unwrap_or_else(|_| "/Users/maroed/RustroverProjects/jingle/jingle_python/.venv/lib/python3.13/site-packages/z3/lib/".to_string());
    // Get the directory where Python's Z3 library is located
    let z3_python_lib = std::path::Path::new(&var);

    // Set the environment variable for Rust to use the same library
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:libdir={}", z3_python_lib.display());

    // Optionally, set the rpath for dynamic libraries
    println!("cargo:rustc-link-search=native={}", z3_python_lib.display());
}
