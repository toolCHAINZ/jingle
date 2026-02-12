use crate::JingleSleighError::EmptyInstruction;
use crate::error::JingleSleighError;
pub use crate::ffi::instruction::bridge::Disassembly;
use crate::ffi::instruction::bridge::InstructionFFI;
use crate::pcode::PcodeOperation;
use crate::{OpCode, VarNode};
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

    /// Performs augmentations to raw pcode to make modeling easier:
    /// * Adds an explicit jump instruction representing fall-through behavior
    /// * Adds call/callother argument/calling convention metadata
    ///
    /// NOTE: This variant accepts the full SleighContext so we can consult the
    /// calling-convention defaults (extrapop) and apply them to emitted CALL/CALLOTHER
    /// operations when no per-site override is present in the ModelingMetadata.
    pub fn postprocess(&mut self, ctx: &crate::context::SleighContext) {
        // Local aliases
        let m = &ctx.metadata;
        let cc_info = ctx.calling_convention_info();
        // Default extrapop from calling-convention, if present
        let default_extrapop: Option<i32> = cc_info
            .default_calling_convention()
            .and_then(|p| p.extrapop);

        // First pass: apply ModelingMetadata overrides (function and callother signatures)
        for op in self.ops.iter_mut() {
            match op {
                PcodeOperation::Call {
                    dest: input,
                    call_info,
                    args,
                } => {
                    // Apply per-address function signature metadata if available
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
                    // Apply per-signature callother metadata if available
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

        // Second pass: ensure CALL / CALLOTHER have extrapop metadata when not explicitly set
        if let Some(def_ep) = default_extrapop {
            for op in self.ops.iter_mut() {
                match op {
                    PcodeOperation::Call { call_info, .. }
                    | PcodeOperation::CallOther { call_info, .. } => {
                        match call_info {
                            Some(ci) => {
                                if ci.extrapop.is_none() {
                                    // clone-modify to avoid borrowing issues
                                    let mut new_ci = ci.clone();
                                    new_ci.extrapop = Some(def_ep);
                                    *call_info = Some(new_ci);
                                }
                            }
                            None => {
                                // Create a minimal CallInfo carrying extrapop for downstream modeling.
                                let new_ci = crate::context::CallInfo {
                                    args: Vec::new(),
                                    outputs: None,
                                    model_behavior: crate::context::ModelingBehavior::default(),
                                    extrapop: Some(def_ep),
                                };
                                *call_info = Some(new_ci);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Push fall-through branch using the SleighContext's arch_info
        let arch_info = ctx.arch_info();
        self.ops.push(PcodeOperation::Branch {
            input: VarNode {
                space_index: arch_info.default_code_space_index(),
                offset: self.address + self.length as u64,
                size: 1,
            },
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::SleighContextBuilder;
    use crate::tests::SLEIGH_ARCH;
    use std::path::Path;

    // Ensure default extrapop from calling convention is applied to CALL ops that lack per-site overrides
    #[test]
    fn test_default_extrapop_applied_to_call() {
        // Build a real SleighContext (tests in this crate use a local Ghidra checkout)
        let builder =
            SleighContextBuilder::load_ghidra_installation(Path::new("/Applications/ghidra"))
                .unwrap();
        let ctx = builder.build(SLEIGH_ARCH).unwrap();

        // Create a Call instruction with no per-site CallInfo
        let dest = VarNode {
            space_index: ctx.arch_info().default_code_space_index(),
            offset: 0x1000,
            size: 8,
        };
        let mut instr = Instruction {
            disassembly: Disassembly {
                mnemonic: "CALL".to_string(),
                args: "".to_string(),
            },
            ops: vec![PcodeOperation::Call {
                dest: dest.clone(),
                args: Vec::new(),
                call_info: None,
            }],
            length: 5,
            address: 0x2000,
        };

        // Run postprocess to attach default calling-convention extrapop
        instr.postprocess(&ctx);

        // Verify the CALL now carries a CallInfo with an extrapop set (from default prototype)
        match &instr.ops[0] {
            PcodeOperation::Call { call_info, .. } => {
                assert!(
                    call_info.is_some(),
                    "Expected call_info to be present after postprocess"
                );
                let ci = call_info.as_ref().unwrap();
                assert!(
                    ci.extrapop.is_some(),
                    "Expected extrapop to be applied from default calling convention"
                );
            }
            _ => panic!("Expected first op to be Call"),
        }
    }

    // Ensure per-site override of extrapop is preserved and not overwritten by defaults
    #[test]
    fn test_per_site_extrapop_override_preserved() {
        // Build a real SleighContext
        let builder =
            SleighContextBuilder::load_ghidra_installation(Path::new("/Applications/ghidra"))
                .unwrap();
        let mut ctx = builder.build(SLEIGH_ARCH).unwrap();

        // Insert per-address CallInfo override for address 0x1000
        let override_addr: u64 = 0x1000;
        let override_extrapop: i32 = 123;
        let call_info_override = crate::context::CallInfo {
            args: Vec::new(),
            outputs: None,
            model_behavior: crate::context::ModelingBehavior::default(),
            extrapop: Some(override_extrapop),
        };
        ctx.metadata.add_call_def(override_addr, call_info_override);

        // Build a Call instruction that targets the address we overrode
        let dest = VarNode {
            space_index: ctx.arch_info().default_code_space_index(),
            offset: override_addr,
            size: 8,
        };
        let mut instr = Instruction {
            disassembly: Disassembly {
                mnemonic: "CALL".to_string(),
                args: "".to_string(),
            },
            ops: vec![PcodeOperation::Call {
                dest: dest.clone(),
                args: Vec::new(),
                call_info: None,
            }],
            length: 5,
            address: 0x2000,
        };

        // Run postprocess which should apply the per-site override (and not overwrite it)
        instr.postprocess(&ctx);

        // Verify the override is preserved
        match &instr.ops[0] {
            PcodeOperation::Call { call_info, .. } => {
                assert!(
                    call_info.is_some(),
                    "Expected call_info to be present after postprocess"
                );
                let ci = call_info.as_ref().unwrap();
                assert_eq!(
                    ci.extrapop,
                    Some(override_extrapop),
                    "Expected per-site extrapop override to be preserved"
                );
            }
            _ => panic!("Expected first op to be Call"),
        }
    }
}
