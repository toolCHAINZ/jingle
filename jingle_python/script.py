import z3
from jingle import *
sleigh = create_sleigh_context("/Users/maroed/RustroverProjects/code_reuse_synthesis_artifacts/crackers/libz.so.1", "/Applications/ghidra")
j = sleigh.make_jingle_context()
state = State(j)

print(state.register("RAX") == state.ram(0x8000_0000, 8))