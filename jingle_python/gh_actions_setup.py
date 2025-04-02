#!/usr/bin/env python3

import shutil
import subprocess
import sys
import os
import platform

ENV_FILE = ".z3env"

def is_command_available(cmd):
    return shutil.which(cmd) is not None

def install_with_yum():
    print("Detected yum. Installing clang and python3-pip...", file=sys.stderr)
    try:
        subprocess.run(['yum', 'install', '-y', 'clang', 'python3-pip'], check=True)
        print("clang and pip installed successfully.", file=sys.stderr)
    except subprocess.CalledProcessError:
        print("Failed to install clang or pip using yum.", file=sys.stderr)
        return

def install_with_apt():
    print("Detected apt. Installing llvm-dev, libclang-dev, clang, and python3-pip...", file=sys.stderr)
    try:
        subprocess.run(['apt', 'update'], check=True)
        subprocess.run(['apt', 'install', '-y', 'build-essential', 'libc6-dev', 'gcc-multilib', 'python3-pip'], check=True)
        print("Packages installed successfully via apt.", file=sys.stderr)
    except subprocess.CalledProcessError:
        print("Failed to install packages using apt.", file=sys.stderr)

def install_pip():
    # Try to install pip if it is not installed
    print("Installing pip...", file=sys.stderr)
    try:
        subprocess.run([sys.executable, '-m', 'ensurepip', '--upgrade'], check=True)
        print("pip installed successfully.", file=sys.stderr)
    except subprocess.CalledProcessError:
        print("Failed to install pip.", file=sys.stderr)
        sys.exit(1)

def write_env_file(header_path):
    with open(ENV_FILE, "w") as f:
        f.write(f"export Z3_SYS_Z3_HEADER={header_path}\n")
    print(f"\n✅ Z3 installed successfully.")
    print(f"💾 Environment variable written to `{ENV_FILE}`.")
    print(f"👉 To load it into your shell, run:\n")
    print(f"    source ./load-z3-env.sh\n")

def install_z3_with_pip():
    print("Installing Z3 from PyPI via pip...", file=sys.stderr)

    # Install the `z3-solver` package directly from PyPI
    subprocess.run([sys.executable, "-m", "pip", "install", "z3-solver"], check=True)
    print("Z3 installed successfully using pip.", file=sys.stderr)

def main():
    # Ensure pip is installed
    try:
        import pip
    except ImportError:
        install_pip()

    # Check if we have a package manager and install necessary tools
    if is_command_available('yum'):
        install_with_yum()
        install_z3_with_pip()
    elif is_command_available('apt'):
        install_with_apt()
        install_z3_with_pip()
    else:
        print("❌ Neither yum nor apt found on this system.", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()
