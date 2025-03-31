import z3
from jingle import *
sleigh = create_sleigh_context("/Users/maroed/RustroverProjects/code_reuse_synthesis_artifacts/crackers/libz.so.1", "/Applications/ghidra")


jingle = sleigh.make_jingle_context()
model = jingle.model_instruction_at(0xa840)
print(z3.simplify(model.original_state.register("RAX")))
print(z3.simplify(model.final_state.register("RAX")))
