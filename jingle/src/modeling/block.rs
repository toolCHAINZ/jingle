use crate::JingleError::EmptyBlock;
use crate::error::JingleError;
use crate::error::JingleError::DisassemblyLengthBound;
use crate::modeling::branch::BranchConstraint;
use crate::modeling::state::State;
use crate::modeling::{ModelingContext, TranslationContext};
use crate::varnode::ResolvedVarnode;
use jingle_sleigh::SpaceInfo;
use jingle_sleigh::{Instruction, VarNode};
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use std::borrow::Borrow;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use z3::{Context, Translate};

/// A `jingle` model of a basic block
#[derive(Debug, Clone)]
pub struct ModeledBlock {
    info: SleighArchInfo,
    pub instructions: Vec<Instruction>,
    state: State,
    original_state: State,
    branch_constraint: BranchConstraint,
    inputs: HashSet<ResolvedVarnode>,
    outputs: HashSet<ResolvedVarnode>,
}

impl Display for ModeledBlock {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        for x in self.instructions.iter() {
            writeln!(f, "{:x} {}", x.address, x.disassembly)?;
        }
        Ok(())
    }
}

impl<T: ModelingContext> TryFrom<&[T]> for ModeledBlock {
    type Error = JingleError;
    fn try_from(vec: &[T]) -> Result<Self, Self::Error> {
        let info = vec.first().ok_or(EmptyBlock)?.get_arch_info();
        let original_state = State::new(info);
        let state = original_state.clone();
        let mut new_block: Self = Self {
            info: info.clone(),
            instructions: Default::default(),
            state,
            original_state,
            inputs: Default::default(),
            outputs: Default::default(),
            branch_constraint: BranchConstraint::with_same_final_branch(
                vec.last().ok_or(EmptyBlock)?.get_branch_constraint(),
            ),
        };

        for ctx in vec {
            for op in ctx.get_ops() {
                new_block.model_pcode_op(op)?;
            }
        }
        Ok(new_block)
    }
}

impl ModeledBlock {
    pub fn read<T: Iterator<Item = Instruction>, S: Borrow<SleighArchInfo>>(
        info: S,
        instr_iter: T,
    ) -> Result<Self, JingleError> {
        let info = info.borrow().clone();
        let original_state = State::new(&info);
        let state = original_state.clone();

        let mut block_terminated = false;
        let mut ops = Vec::new();
        let mut instructions = Vec::new();
        // The block_terminated check ensures that this function will only return successfully
        // in cases where this has been initialized with an actual value.
        let mut naive_fallthrough_address: u64 = 0;
        for instr in instr_iter {
            ops.extend_from_slice(&instr.ops);
            if instr.terminates_basic_block() {
                block_terminated = true;
                naive_fallthrough_address = instr.next_addr();
            }
            instructions.push(instr);
            if block_terminated {
                break;
            }
        }
        if !block_terminated {
            return Err(DisassemblyLengthBound);
        }
        let vn = state.get_default_code_space_info().make_varnode(
            naive_fallthrough_address,
            state.get_default_code_space_info().index_size_bytes as usize,
        );

        let mut model = Self {
            info: info,
            instructions,
            state,
            original_state,
            branch_constraint: BranchConstraint::new(&vn),
            inputs: Default::default(),
            outputs: Default::default(),
        };
        for op in ops {
            model.model_pcode_op(&op)?
        }
        Ok(model)
    }

    pub fn fresh(&self) -> Result<Self, JingleError> {
        ModeledBlock::read(&self.info, self.instructions.clone().into_iter())
    }

    pub fn get_first_address(&self) -> u64 {
        self.instructions[0].address
    }

    pub fn get_last_address(&self) -> u64 {
        let i = self.instructions.last().unwrap();
        i.address + i.length as u64
    }
}

impl ModelingContext for ModeledBlock {
    fn get_arch_info(&self) -> &SleighArchInfo {
        &self.info
    }

    fn get_address(&self) -> u64 {
        self.instructions[0].address
    }

    fn get_original_state(&self) -> &State {
        &self.original_state
    }

    fn get_final_state(&self) -> &State {
        &self.state
    }

    fn get_ops(&self) -> Vec<&PcodeOperation> {
        let mut result = vec![];
        for instr in self.instructions.iter() {
            for x in instr.ops.iter() {
                result.push(x);
            }
        }
        result
    }

    fn get_inputs(&self) -> HashSet<ResolvedVarnode> {
        self.inputs.clone()
    }

    fn get_outputs(&self) -> HashSet<ResolvedVarnode> {
        self.outputs.clone()
    }

    fn get_branch_constraint(&self) -> &BranchConstraint {
        &self.branch_constraint
    }
}

impl TranslationContext for ModeledBlock {
    fn track_input(&mut self, input: &ResolvedVarnode) {
        self.inputs.insert(input.clone());
    }
    fn track_output(&mut self, output: &ResolvedVarnode) {
        self.outputs.insert(output.clone());
    }

    fn get_final_state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    fn get_branch_builder(&mut self) -> &mut BranchConstraint {
        &mut self.branch_constraint
    }
}

unsafe impl Translate for ModeledBlock {
    fn translate(&self, dest: &Context) -> Self {
        Self {
            info: self.info.clone(),
            branch_constraint: self.branch_constraint.clone(),
            original_state: self.original_state.translate(dest),
            state: self.state.translate(dest),
            inputs: self.inputs.iter().map(|a| a.translate(dest)).collect(),
            instructions: self.instructions.clone(),
            outputs: self.outputs.iter().map(|a| a.translate(dest)).collect(),
        }
    }
}
