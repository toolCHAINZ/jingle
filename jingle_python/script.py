import z3
from z3 import simplify
from jingle import *
sleigh = create_sleigh_context("/Users/denhomc1/PycharmProjects/code_reuse_synthesis_artifacts/crackers/libz.so.1", "/Applications/ghidra")


jingle = sleigh.make_jingle_context()
model = jingle.model_block_at(0xa860, 9)
print(simplify(model.final_state.register("RSP") == model.original_state.register("RSP")))
print(simplify(model.final_state.register("RAX") == model.original_state.register("RAX")))
print(simplify(model.final_state.register("RBX") == model.original_state.register("RBX")))
