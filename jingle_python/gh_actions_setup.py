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

def write_env_file(z3_header_path, z3_lib_path):
    with open(ENV_FILE, "w") as f:
        f.write(f"export Z3_SYS_Z3_HEADER={z3_header_path}\n")
        f.write(f"export LD_LIBRARY_PATH={z3_lib_path}\n")
    print(f"\n✅ Z3 environment variables written to `{ENV_FILE}`.")
    print(f"👉 To load them into your shell, run:\n")
    print(f"    source ./{ENV_FILE}\n")

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
        plat = "x86_64-unknown-linux-gnu"
    elif target_platform == 'aarch64':  # ARM64 architecture
        wheel_arch_prefix = "manylinux_2_34_aarch64"
        plat = "aarch64-unknown-linux-gnu"
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
    subprocess.run(["uv", "pip", "install", wheel_url, "--python-platform", plat], check=True)

    # After installation, we'll look for the z3 header and library locations
    # This can be adjusted to wherever the files get installed by pip,
    # but typically these will be within site-packages
    import site
    site_packages_path = site.getsitepackages()[0]  # Get the site-packages directory
    z3_header_path = os.path.join(site_packages_path, "z3", "include", "z3.h")
    z3_lib_path = os.path.join(site_packages_path, "z3", "lib")

    # Write the paths to the environment file
    write_env_file(z3_header_path, z3_lib_path)

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
