#!/usr/bin/env python3

import shutil
import subprocess
import sys
import os
import tempfile
import urllib.request
import json
import zipfile
import platform

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

def write_env_file(header_path):
    with open(ENV_FILE, "w") as f:
        f.write(f"export Z3_SYS_Z3_HEADER={header_path}\n")
        f.write(f"export LD_LIBRARY_PATH=/usr/local/lib\n")
    print(f"\n✅ Z3 installed successfully.")
    print(f"💾 Environment variable written to `{ENV_FILE}`.")
    print(f"👉 To load it into your shell, run:\n")
    print(f"    source ./load-z3-env.sh\n")

def install_z3_latest():
    print("Fetching and installing Z3 from GitHub...", file=sys.stderr)
    api_url = "https://api.github.com/repos/Z3Prover/z3/releases/latest"
    with urllib.request.urlopen(api_url) as response:
        data = json.loads(response.read().decode())

    assets = data.get("assets", [])

    # Detect the current architecture
    arch = platform.machine()
    print(f"Detected architecture: {arch}")

    # Select appropriate artifact based on architecture
    if arch == 'x86_64':  # x64 architecture
        artifact_name = "x64-glibc"
    elif arch == 'aarch64':  # ARM64 architecture
        artifact_name = "arm64-glibc"
    else:
        raise Exception(f"Unsupported architecture: {arch}")

    # Find the correct zip URL based on architecture
    zip_url = None
    for asset in assets:
        if artifact_name in asset["name"] and asset["name"].endswith(".zip"):
            zip_url = asset["browser_download_url"]
            break

    if not zip_url:
        raise Exception(f"Could not find a suitable {artifact_name} Z3 zip archive.")

    with tempfile.TemporaryDirectory() as tmpdir:
        zip_path = os.path.join(tmpdir, "z3.zip")
        urllib.request.urlretrieve(zip_url, zip_path)

        print("Extracting Z3 archive...", file=sys.stderr)
        with zipfile.ZipFile(zip_path, "r") as zip_ref:
            zip_ref.extractall(tmpdir)

        extracted_dirs = [d for d in os.listdir(tmpdir) if os.path.isdir(os.path.join(tmpdir, d))]
        if not extracted_dirs:
            raise Exception("Extraction failed or directory structure unexpected.")
        z3_dir = os.path.join(tmpdir, extracted_dirs[0])

        include_dir = os.path.join(z3_dir, "include")
        lib_dir = os.path.join(z3_dir, "bin")

        print("Installing headers and libraries to /usr/local...", file=sys.stderr)
        subprocess.run(['mkdir', '-p', '/usr/local/include/z3'], check=True)
        subprocess.run(['cp', '-r', include_dir + '/', '/usr/local/include/z3'], check=True)
        subprocess.run(['cp', os.path.join(lib_dir, 'libz3.so'), '/usr/local/lib/'], check=True)
        subprocess.run(['ls', '/usr/local/lib'], check=True)
        subprocess.run(['ldconfig'], check=True)

        z3_header_path = '/usr/local/include/z3/include/z3.h'
        if os.path.exists(z3_header_path):
            write_env_file(z3_header_path)
        else:
            print("⚠️ Warning: z3.h not found at expected path.", file=sys.stderr)

def main():
    if is_command_available('yum'):
        install_with_yum()
        install_z3_latest()
    elif is_command_available('apt'):
        install_with_apt()
        install_z3_latest()
    else:
        print("❌ Neither yum nor apt found on this system.", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
