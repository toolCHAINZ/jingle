#!/usr/bin/env python3

import shutil
import subprocess
import sys
import os
import urllib.request
import json
import argparse
from urllib.parse import urljoin

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

def write_env_file():
    print(f"\n✅ Z3 installed successfully via Python wheel.")
    print(f"💾 Environment variable written to `{ENV_FILE}`.")
    print(f"👉 To load it into your shell, run:\n")
    print(f"    source ./load-z3-env.sh\n")

def install_z3_wheel(target_platform):
    print(f"Fetching and installing Z3 Python wheel for target platform '{target_platform}'...", file=sys.stderr)

    # GitHub API to fetch the latest release
    api_url = "https://api.github.com/repos/Z3Prover/z3/releases/latest"
    with urllib.request.urlopen(api_url) as response:
        data = json.loads(response.read().decode())

    assets = data.get("assets", [])

    # Define wheel file prefix based on target platform
    if target_platform == 'x86_64':  # x64 architecture
        wheel_arch_prefix = "manylinux_2_17_x86_64.manylinux2014_x86_64"
    elif target_platform == 'aarch64':  # ARM64 architecture
        wheel_arch_prefix = "manylinux_2_34_aarch64"
    else:
        raise Exception(f"Unsupported target platform: {target_platform}")

    # Search for the correct wheel file in the release assets
    wheel_url = None
    for asset in assets:
        if "py3-none-manylinux" in asset["name"] and wheel_arch_prefix in asset["name"] and asset["name"].endswith(".whl"):
            wheel_url = asset["browser_download_url"]
            break

    if not wheel_url:
        raise Exception(f"Could not find a suitable Z3 wheel for platform {target_platform}.")

    # Download and install the wheel using pip
    print(f"Downloading wheel from {wheel_url}...", file=sys.stderr)
    subprocess.run([sys.executable, "-m", "pip", "install", wheel_url], check=True)

    # Set environment variables (optional if needed for your setup)
    write_env_file()

def main():
    parser = argparse.ArgumentParser(description="Install Z3 Python wheel for a target platform.")
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

    # Install Z3 wheel based on the target platform
    install_z3_wheel(args.target_platform)

if __name__ == "__main__":
    main()
