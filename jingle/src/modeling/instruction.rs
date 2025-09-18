use crate::modeling::{ModelingContext, TranslationContext};
use jingle_sleigh::Instruction;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use std::borrow::Borrow;

use crate::modeling::branch::BranchConstraint;
use crate::modeling::state::State;
use std::collections::HashSet;
use z3::{Context, Translate};

use crate::JingleError;
use crate::varnode::ResolvedVarnode;

/// A `jingle` model of an individual SLEIGH instruction
#[derive(Debug, Clone)]
pub struct ModeledInstruction {
    info: SleighArchInfo,
    pub instr: Instruction,
    state: State,
    original_state: State,
    inputs: HashSet<ResolvedVarnode>,
    outputs: HashSet<ResolvedVarnode>,
    branch_builder: BranchConstraint,
}

impl ModeledInstruction {
    pub fn new<T: Borrow<SleighArchInfo>>(
        instr: Instruction,
        info: T,
    ) -> Result<Self, JingleError> {
        let info = info.borrow().clone();
        let original_state = State::new(&info);
        let state = original_state.clone();
        let next_vn = state.get_default_code_space_info().make_varnode(
            instr.next_addr(),
            state.get_default_code_space_info().index_size_bytes as usize,
        );
        let mut model = Self {
            info: info.borrow().clone(),
            instr,
            state,
            original_state,
            inputs: Default::default(),
            outputs: Default::default(),
            branch_builder: BranchConstraint::new(&next_vn),
        };
        for x in model.instr.clone().ops.iter() {
            model.model_pcode_op(x)?;
        }
        Ok(model)
    }

    pub fn fresh(&self) -> Result<Self, JingleError> {
        ModeledInstruction::new(self.instr.clone(), &self.info)
    }
}
impl ModelingContext for ModeledInstruction {
    fn get_arch_info(&self) -> &SleighArchInfo {
        &self.info
    }

    fn get_address(&self) -> u64 {
        self.instr.address
    }

    fn get_original_state(&self) -> &State {
        &self.original_state
    }

    fn get_final_state(&self) -> &State {
        &self.state
    }

    fn get_ops(&self) -> Vec<&PcodeOperation> {
        let mut result = vec![];
        for x in self.instr.ops.iter() {
            result.push(x);
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
        &self.branch_builder
    }
}

unsafe impl Translate for ModeledInstruction {
    fn translate(&self, dest: &Context) -> Self {
        Self {
            info: self.info.clone(),
            instr: self.instr.clone(),
            state: self.state.translate(dest),
            original_state: self.state.translate(dest),
            inputs: self.inputs.clone(),
            outputs: self.outputs.clone(),
            branch_builder: self.branch_builder.clone(),
        }
    }
}

impl TranslationContext for ModeledInstruction {
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
        &mut self.branch_builder
    }
}
