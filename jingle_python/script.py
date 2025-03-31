import z3
from jingle import *
sleigh = create_sleigh_context("/Users/maroed/RustroverProjects/code_reuse_synthesis_artifacts/crackers/libz.so.1", "/Applications/ghidra")

print(sleigh.base_address)
sleigh.base_address = 300
print(sleigh.base_address)
