#!/usr/bin/env python3

import shutil
import subprocess
import sys
import os
import tempfile
import urllib.request
import json
import tarfile

def is_command_available(cmd):
    return shutil.which(cmd) is not None

def install_with_yum():
    print("Detected yum. Installing clang...")
    try:
        subprocess.run(['yum', 'install', '-y', 'clang'], check=True)
        print("clang installed successfully.")
    except subprocess.CalledProcessError:
        print("Failed to install clang using yum.")
        return

    print("Fetching and installing Z3 from GitHub...")
    try:
        install_z3_latest()
    except Exception as e:
        print(f"Failed to install Z3: {e}")

def install_with_apt():
    print("Detected apt. Installing llvm-dev, libclang-dev, clang, and z3...")
    try:
        subprocess.run(['apt', 'update'], check=True)
        subprocess.run(['apt', 'install', '-y', 'llvm-dev', 'libclang-dev', 'clang', "libz3-dev"], check=True)
        print("Packages installed successfully via apt.")
    except subprocess.CalledProcessError:
        print("Failed to install packages using apt.")

def install_z3_latest():
    # Get the latest Z3 release info
    api_url = "https://api.github.com/repos/Z3Prover/z3/releases/latest"
    with urllib.request.urlopen(api_url) as response:
        data = json.loads(response.read().decode())

    # Look for a Linux x64 tarball
    assets = data.get("assets", [])
    tarball_url = None
    for asset in assets:
        if "x64-linux" in asset["name"] and asset["name"].endswith(".tar.gz"):
            tarball_url = asset["browser_download_url"]
            break

    if not tarball_url:
        raise Exception("Could not find a suitable Linux x64 Z3 tarball.")

    print(f"Downloading: {tarball_url}")
    with tempfile.TemporaryDirectory() as tmpdir:
        tar_path = os.path.join(tmpdir, "z3.tar.gz")
        urllib.request.urlretrieve(tarball_url, tar_path)

        print("Extracting tarball...")
        with tarfile.open(tar_path, "r:gz") as tar:
            tar.extractall(path=tmpdir)

        # Find the extracted directory
        extracted_dirs = [d for d in os.listdir(tmpdir) if os.path.isdir(os.path.join(tmpdir, d))]
        if not extracted_dirs:
            raise Exception("Extraction failed or directory structure unexpected.")
        z3_dir = os.path.join(tmpdir, extracted_dirs[0])

        include_dir = os.path.join(z3_dir, "include")
        lib_dir = os.path.join(z3_dir, "bin")  # Z3 often puts .so files in 'bin'

        # Install to /usr/local
        print("Installing headers and shared libraries to /usr/local...")
        subprocess.run(['cp', '-r', include_dir, '/usr/local/include/z3'], check=True)
        subprocess.run(['cp', os.path.join(lib_dir, 'libz3.so'), '/usr/local/lib/'], check=True)

        # Refresh the linker cache
        subprocess.run(['ldconfig'], check=True)

    print("Z3 installed successfully.")

def main():
    if is_command_available('yum'):
        install_with_yum()
    elif is_command_available('apt'):
        install_with_apt()
    else:
        print("Neither yum nor apt found on this system.")
        sys.exit(1)

if __name__ == "__main__":
    main()
