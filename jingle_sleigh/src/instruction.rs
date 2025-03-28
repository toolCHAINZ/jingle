use crate::display::PcodeOperationDisplay;
use crate::error::JingleSleighError;
pub use crate::ffi::instruction::bridge::Disassembly;
use crate::ffi::instruction::bridge::InstructionFFI;
use crate::pcode::PcodeOperation;
use crate::JingleSleighError::EmptyInstruction;
use crate::{ArchInfoProvider, OpCode};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
#[cfg(feature = "pyo3")]
use pyo3::pyclass;

/// A rust representation of a SLEIGH assembly instruction
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "pyo3", pyclass)]
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

/// A helper structure allowing displaying an instruction and its semantics
/// without requiring lots of pcode metadata to be stored in the instruction itself
pub struct InstructionDisplay<'a, T: ArchInfoProvider> {
    pub disassembly: Disassembly,
    pub ops: Vec<PcodeOperationDisplay<'a, T>>,
}

impl Instruction {
    pub fn display<'a, T: ArchInfoProvider>(
        &'a self,
        ctx: &'a T,
    ) -> Result<InstructionDisplay<'a, T>, JingleSleighError> {
        let mut ops: Vec<PcodeOperationDisplay<T>> = Vec::with_capacity(self.ops.len());
        for x in &self.ops {
            ops.push(x.display(ctx)?)
        }
        Ok(InstructionDisplay {
            disassembly: self.disassembly.clone(),
            ops,
        })
    }

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
}

impl<T: ArchInfoProvider> Display for InstructionDisplay<'_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {}", self.disassembly.mnemonic, self.disassembly.args)?;
        for x in &self.ops {
            writeln!(f, "{}", x)?;
        }
        Ok(())
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
