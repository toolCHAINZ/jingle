import z3
from jingle import *
sleigh = create_sleigh_context("/Users/maroed/RustroverProjects/code_reuse_synthesis_artifacts/crackers/libz.so.1", "/Applications/ghidra")


jingle = sleigh.make_jingle_context()
model = jingle.model_block_at(0xa860, 9)
for output in model.get_input_bvs():
    print(z3.simplify(output))