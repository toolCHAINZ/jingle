import z3
from jingle import *
sleigh = create_sleigh_context("/Users/maroed/RustroverProjects/code_reuse_synthesis_artifacts/crackers/libz.so.1", "/Applications/ghidra")
j = sleigh.make_jingle_context()
state = State(j)
bv = state.read_varnode(VarNode(2, 100, 2))
print(bv + bv)