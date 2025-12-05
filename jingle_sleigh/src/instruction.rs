use crate::OpCode;
use crate::error::JingleSleighError;
pub use crate::ffi::instruction::bridge::Disassembly;
use crate::ffi::instruction::bridge::InstructionFFI;
use crate::pcode::PcodeOperation;
use crate::{JingleSleighError::EmptyInstruction, context::loaded::ModelingMetadata};
use serde::{Deserialize, Serialize};

/// A rust representation of a SLEIGH assembly instruction
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Instruction {
    pub disassembly: Disassembly,
    /// The PCODE semantics of this instruction
    /// todo: this should someday be a graph instead of a vec
    pub ops: Vec<PcodeOperation>,
    /// The number of bytes taken up by the encoding of this assembly instruction
    pub length: usize,
    /// The address this instruction was read from
    pub address: u64,
}

impl Instruction {
    pub fn next_addr(&self) -> u64 {
        self.address + self.length as u64
    }

    pub fn ops_equal(&self, other: &Self) -> bool {
        self.ops.eq(&other.ops)
    }
    pub fn terminates_basic_block(&self) -> bool {
        self.ops.iter().any(|o| o.terminates_block())
    }

    pub fn has_syscall(&self) -> bool {
        self.ops
            .iter()
            .any(|o| o.opcode() == OpCode::CPUI_CALLOTHER)
    }

    pub fn augment_with_metadata(&mut self, m: &ModelingMetadata) {
        for op in self.ops.iter_mut() {
            match op {
                PcodeOperation::Call {
                    dest: input,
                    call_info,
                    args,
                } => {
                    if let Some(a) = m.func_info.get(&input.offset) {
                        *call_info = Some(a.clone());
                        for ele in &a.args {
                            args.push(ele.clone());
                        }
                    }
                }
                PcodeOperation::CallOther {
                    inputs, call_info, ..
                } => {
                    if let Some(a) = m.callother_info.get(inputs) {
                        *call_info = Some(a.clone());
                        for ele in &a.args {
                            inputs.push(ele.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl From<InstructionFFI> for Instruction {
    fn from(value: InstructionFFI) -> Self {
        let ops = value.ops.into_iter().map(PcodeOperation::from).collect();
        Instruction {
            disassembly: value.disassembly,
            ops,
            length: value.length,
            address: value.address,
        }
    }
}

/// todo: this is a gross placeholder until I refactor stuff into a proper
/// trace
impl TryFrom<&[Instruction]> for Instruction {
    type Error = JingleSleighError;
    fn try_from(value: &[Instruction]) -> Result<Self, JingleSleighError> {
        if value.is_empty() {
            return Err(EmptyInstruction);
        }
        if value.len() == 1 {
            return Ok(value[0].clone());
        }
        let ops: Vec<PcodeOperation> = value.iter().flat_map(|i| i.ops.iter().cloned()).collect();
        let length = value.iter().map(|i| i.length).reduce(|a, b| a + b).unwrap();
        let address = value[0].address;
        let disassembly = Disassembly {
            mnemonic: "<multiple instructions>".to_string(),
            args: "".to_string(),
        };
        Ok(Self {
            ops,
            length,
            address,
            disassembly,
        })
    }
}
