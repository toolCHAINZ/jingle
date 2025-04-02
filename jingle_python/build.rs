use std::process::Command;

fn main() {
    // Run the Python script to get the venv's lib directory
    let output = Command::new("python3")
        .arg("find_venv_library_path.py")
        .output()
        .expect("Failed to execute Python script");

    if !output.status.success() {
        println!("WARNING: not linking with python's z3");
    } else {
        let venv_lib = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let z3_python_lib = std::path::Path::new(&venv_lib).join("z3").join("lib");
        println!("cargo:rustc-link-search=native={}", z3_python_lib.display());
    }
}
