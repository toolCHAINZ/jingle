use std::process::Command;

fn main() {
    // Run the Python script to get the venv's lib directory
    let output = Command::new("python")
        .arg("find_path.py") // Assuming the script is named find_venv_lib.py
        .output()
        .expect("Failed to execute Python script");

    if !output.status.success() {
        panic!("Python script failed: {:?}", output);
    }

    let venv_lib = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Get the directory where Python's Z3 library is located
    let z3_python_lib = std::path::Path::new(&venv_lib).join("z3").join("lib");
    // Optionally, set the rpath for dynamic libraries
    println!("cargo:rustc-link-search=native={}", z3_python_lib.display());
}
