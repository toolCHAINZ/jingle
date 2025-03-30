import sys
import os
import sysconfig

# Get the virtual environment's lib directory
venv_lib = sysconfig.get_paths()["purelib"]

# Print the path so that we can capture it in build.rs
print(venv_lib)