#!/usr/bin/env python3

import shutil
import subprocess
import sys
import os
import urllib.request
import json
import argparse
import tempfile
import zipfile

ENV_FILE = ".z3env"

def is_command_available(cmd):
    return shutil.which(cmd) is not None

def install_with_yum():
    print("Detected yum. Installing clang...", file=sys.stderr)
    try:
        subprocess.run(['yum', 'install', '-y', 'clang'], check=True)
        print("clang installed successfully.", file=sys.stderr)
    except subprocess.CalledProcessError:
        print("Failed to install clang using yum.", file=sys.stderr)
        return

def install_with_apt():
    print("Detected apt. Installing llvm-dev, libclang-dev, and clang...", file=sys.stderr)
    try:
        subprocess.run(['apt', 'update'], check=True)
        subprocess.run(['apt', 'install', '-y', 'build-essential', 'libc6-dev', 'gcc-multilib'], check=True)
        print("Packages installed successfully via apt.", file=sys.stderr)
    except subprocess.CalledProcessError:
        print("Failed to install packages using apt.", file=sys.stderr)

def write_env_file(z3_header_path, z3_lib_path):
    with open(ENV_FILE, "w") as f:
        f.write(f"export Z3_SYS_Z3_HEADER={z3_header_path}\n")
        f.write(f"export LD_LIBRARY_PATH={z3_lib_path}")
        f.write(f"export Z3_PATH={z3_lib_path}/libz3.so")
        f.write(f"export RUSTFLAGS='-L native={z3_lib_path}'")
    print(f"\n✅ Z3 environment variables written to `{ENV_FILE}`.")
    print(f"👉 To load them into your shell, run:\n")
    print(f"    source ./{ENV_FILE}\n")

def install_z3_glibc(target_platform):
    print(f"Fetching and installing Z3 Linux glibc build for target platform '{target_platform}'...", file=sys.stderr)

    # GitHub API to fetch the latest release
    api_url = "https://api.github.com/repos/Z3Prover/z3/releases/latest"
    with urllib.request.urlopen(api_url) as response:
        data = json.loads(response.read().decode())

    assets = data.get("assets", [])

    # Define glibc build prefix based on target platform
    if target_platform == 'x86_64':  # x64 architecture
        glibc_arch_prefix = "x64-glibc"
    elif target_platform == 'aarch64':  # ARM64 architecture
        glibc_arch_prefix = "arm64-glibc"
    else:
        raise Exception(f"Unsupported target platform: {target_platform}")

    # Search for the correct glibc build file in the release assets
    glibc_url = None
    for asset in assets:
        if glibc_arch_prefix in asset["name"] and asset["name"].endswith(".zip"):
            glibc_url = asset["browser_download_url"]
            break

    if not glibc_url:
        raise Exception(f"Could not find a suitable Z3 glibc build for platform {target_platform}.")

    # Download and extract the glibc build
    print(f"Downloading glibc build from {glibc_url}...", file=sys.stderr)
    with tempfile.TemporaryDirectory() as tmpdir:
        zip_path = os.path.join(tmpdir, "z3-glibc.zip")
        urllib.request.urlretrieve(glibc_url, zip_path)

        print("Extracting Z3 glibc archive...", file=sys.stderr)
        with zipfile.ZipFile(zip_path, "r") as zip_ref:
            zip_ref.extractall(tmpdir)

        # Find the extracted Z3 directory
        extracted_dirs = [d for d in os.listdir(tmpdir) if os.path.isdir(os.path.join(tmpdir, d))]
        if not extracted_dirs:
            raise Exception("Extraction failed or directory structure unexpected.")
        z3_dir = os.path.join(tmpdir, extracted_dirs[0])

        # Set the include and lib directories
        include_dir = os.path.join(z3_dir, "include")
        lib_dir = os.path.join(z3_dir, "bin")

        # Create a stable location for Z3
        stable_install_dir = "/usr/local/z3"
        subprocess.run(['mkdir', '-p', stable_install_dir], check=True)
        subprocess.run(['cp', '-r', include_dir, stable_install_dir], check=True)
        subprocess.run(['cp', os.path.join(lib_dir, 'libz3.so'), '/usr/local/lib/'], check=True)
        subprocess.run(['ldconfig'], check=True)

        # Set paths for z3.h and libz3.so
        z3_header_path = os.path.join(stable_install_dir, "include", "z3.h")
        z3_lib_path = "/usr/local/lib"

        # Write the paths to the environment file
        write_env_file(z3_header_path, z3_lib_path)

def main():
    parser = argparse.ArgumentParser(description="Install Z3 glibc build for a target platform.")
    parser.add_argument("target_platform", choices=["x86_64", "aarch64"],
                        help="Target platform architecture (e.g., x86_64 or aarch64).")
    args = parser.parse_args()

    # Check if 'yum' or 'apt' are available for optional package installation
    if is_command_available('yum'):
        install_with_yum()
    elif is_command_available('apt'):
        install_with_apt()
    else:
        print("❌ Neither yum nor apt found on this system.", file=sys.stderr)

    # Install Z3 glibc build based on the target platform
    install_z3_glibc(args.target_platform)

if __name__ == "__main__":
    main()
