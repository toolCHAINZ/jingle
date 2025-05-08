from typing import Any, Union, List, Optional, Iterable
import z3  # Assuming Z3 is imported for type annotations

class SpaceInfo:
    # Placeholder for SpaceInfo class
    ...

class VarNode:
    def __init__(self, space_index: int, offset: int, size: int) -> None: ...

class RawVarNodeDisplay:
    def __init__(self, offset: int, size: int, space_info: SpaceInfo) -> None: ...

class VarNodeDisplay:
    def __init__(self, raw: RawVarNodeDisplay = ..., register: tuple[str, VarNode] = ...) -> None: ...
    # Represents the enum variants Raw and Register
    raw: RawVarNodeDisplay
    register: tuple[str, VarNode]


class ResolvedIndirectVarNode:
    def __init__(self, pointer: Any, pointer_space_info: SpaceInfo, access_size_bytes: int) -> None: ...

    def pointer_bv(self) -> z3.BitVecRef: ...
    def space_name(self) -> str: ...
    def access_size(self) -> int: ...


class ResolvedVarNode:
    """
    Represents the PythonResolvedVarNode enum with two variants:
    - Direct: Contains a VarNodeDisplay
    - Indirect: Contains a ResolvedIndirectVarNode
    """
    def __init__(self, value: Union[VarNodeDisplay, ResolvedIndirectVarNode]) -> None: ...
    value: Union[VarNodeDisplay, ResolvedIndirectVarNode]


class PcodeOperation:
    pass


class Instruction:
    """
    Represents a Python wrapper for a Ghidra instruction.
    """
    disassembly: str
    def pcode(self) -> List[PcodeOperation]: ...

class State:
    def __init__(self, jingle: JingleContext) -> State: ...

    def varnode(self, varnode: ResolvedVarNode) -> z3.BitVecRef: ...
    def register(self, name: str) -> z3.BitVecRef: ...
    def ram(self, offset: int, length: int) -> z3.BitVecRef: ...

class ModeledInstruction:
    original_state: State
    final_state: State

    def get_input_vns(self) -> Iterable[ResolvedVarNode]: ...
    def get_output_vns(self) -> Iterable[ResolvedVarNode]: ...

class ModeledBlock:
    original_state: State
    final_state: State
    def get_input_vns(self) -> Iterable[ResolvedVarNode]: ...
    def get_output_vns(self) -> Iterable[ResolvedVarNode]: ...

class JingleContext:
    def model_instruction_at(self, offset: int) -> ModeledInstruction: ...
    def model_block_at(self, offset: int, max_instrs: int) -> ModeledBlock: ...

class SleighContext:
    """
    Represents a Sleigh context in python.
    """
    base_address: int
    def instruction_at(self, offset: int) -> Optional[Instruction]: ...
    def make_jingle_context(self) -> JingleContext: ...


def create_sleigh_context(binary_path: str, ghidra: str) -> SleighContext: ...
def create_jingle_context(binary_path: str, ghidra: str) -> JingleContext: ...