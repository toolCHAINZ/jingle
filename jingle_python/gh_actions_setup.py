#!/usr/bin/env python3

import shutil
import subprocess
import sys
import os
import platform
import site

ENV_FILE = ".z3env"

def is_command_available(cmd):
    return shutil.which(cmd) is not None

def install_with_yum():
    print("Detected yum. Installing clang and curl...", file=sys.stderr)
    try:
        subprocess.run(['yum', 'install', '-y', 'clang', 'curl'], check=True)
        print("clang and curl installed successfully.", file=sys.stderr)
    except subprocess.CalledProcessError:
        print("Failed to install clang or curl using yum.", file=sys.stderr)
        return

def install_with_apt():
    print("Detected apt. Installing llvm-dev, libclang-dev, clang, and curl...", file=sys.stderr)
    try:
        subprocess.run(['apt', 'update'], check=True)
        subprocess.run(['apt', 'install', '-y', 'build-essential', 'libc6-dev', 'gcc-multilib', 'curl'], check=True)
        print("Packages installed successfully via apt.", file=sys.stderr)
    except subprocess.CalledProcessError:
        print("Failed to install packages using apt.", file=sys.stderr)

def install_uv():
    # Install uv using curl
    print("Installing uv using curl...", file=sys.stderr)
    try:
        with open("install.sh", "w") as o:
            subprocess.run(['curl', '-LsSf', 'https://astral.sh/uv/install.sh'], check=True, stdout=o)
        subprocess.run(['sh', 'install.sh'], check=True)
        print("uv installed successfully.", file=sys.stderr)
    except subprocess.CalledProcessError:
        print("Failed to install uv.", file=sys.stderr)
        sys.exit(1)

def find_z3_header_path():
    """Find the z3.h header file and return its path."""
    site_packages_dir = site.getsitepackages()[0]
    z3_include_dir = os.path.join(site_packages_dir, "z3-solver", "include", "z3")

    # Check if the z3.h file exists in the expected location
    z3_header_path = os.path.join(z3_include_dir, "z3.h")
    if os.path.exists(z3_header_path):
        return z3_header_path
    else:
        print(f"❌ Could not find z3.h at expected path: {z3_header_path}", file=sys.stderr)
        sys.exit(1)

def write_env_file(header_path):
    with open(ENV_FILE, "w") as f:
        f.write(f"export Z3_SYS_Z3_HEADER={header_path}\n")
    print(f"\n✅ Z3 installed successfully.")
    print(f"💾 Environment variable written to `{ENV_FILE}`.")
    print(f"👉 To load it into your shell, run:\n")
    print(f"    source ./load-z3-env.sh\n")

def install_z3_with_uv():
    print("Installing Z3 from the UV repository via uv...", file=sys.stderr)

    # Install the `z3-solver` package directly using uv
    subprocess.run(["uv", "pip", "install", "--system", "z3-solver"], check=True)
    print("Z3 installed successfully using uv.", file=sys.stderr)

def main():
    # Ensure uv is installed
    if not is_command_available('uv'):
        install_uv()

    # Check if we have a package manager and install necessary tools
    if is_command_available('yum'):
        install_with_yum()
        install_z3_with_uv()
    elif is_command_available('apt'):
        install_with_apt()
        install_z3_with_uv()
    else:
        print("❌ Neither yum nor apt found on this system.", file=sys.stderr)
        sys.exit(1)

    # Find the Z3 header path and set the environment variable
    z3_header_path = find_z3_header_path()
    write_env_file(z3_header_path)

if __name__ == "__main__":
    main()
