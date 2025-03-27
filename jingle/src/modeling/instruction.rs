use crate::modeling::{ModelingContext, TranslationContext};
use jingle_sleigh::PcodeOperation;
use jingle_sleigh::{Instruction, VarNode};

use std::collections::HashSet;

use crate::modeling::branch::BranchConstraint;
use crate::modeling::state::State;

use crate::varnode::ResolvedVarnode;
use crate::{JingleContext, JingleError};
use jingle_sleigh::{ArchInfoProvider, SpaceInfo};

/// A `jingle` model of an individual SLEIGH instruction
#[derive(Debug, Clone)]
pub struct ModeledInstruction<'ctx> {
    jingle: JingleContext<'ctx>,
    pub instr: Instruction,
    state: State<'ctx>,
    original_state: State<'ctx>,
    inputs: HashSet<ResolvedVarnode<'ctx>>,
    outputs: HashSet<ResolvedVarnode<'ctx>>,
    branch_builder: BranchConstraint,
}

impl<'ctx> ModeledInstruction<'ctx> {
    pub fn new(instr: Instruction, jingle: &JingleContext<'ctx>) -> Result<Self, JingleError> {
        let original_state = State::new(jingle);
        let state = original_state.clone();
        let next_vn = state.get_default_code_space_info().make_varnode(
            instr.next_addr(),
            state.get_default_code_space_info().index_size_bytes as usize,
        );
        let mut model = Self {
            jingle: jingle.clone(),
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
        ModeledInstruction::new(self.instr.clone(), &self.jingle)
    }
}

impl ArchInfoProvider for ModeledInstruction<'_> {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.jingle.get_space_info(idx)
    }

    fn get_all_space_info(&self) -> impl Iterator<Item = &SpaceInfo> {
        self.jingle.get_all_space_info()
    }

    fn get_code_space_idx(&self) -> usize {
        self.jingle.get_code_space_idx()
    }

    fn get_register(&self, name: &str) -> Option<&VarNode> {
        self.jingle.get_register(name)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.jingle.get_register_name(location)
    }

    fn get_registers(&self) -> impl Iterator<Item = (&VarNode, &str)> {
        self.jingle.get_registers().map(|(a, b)| (a, b))
    }
}

impl<'ctx> ModelingContext<'ctx> for ModeledInstruction<'ctx> {
    fn get_jingle(&self) -> &JingleContext<'ctx> {
        &self.jingle
    }

    fn get_address(&self) -> u64 {
        self.instr.address
    }

    fn get_original_state(&self) -> &State<'ctx> {
        &self.original_state
    }

    fn get_final_state<'a>(&'a self) -> &'a State<'ctx> {
        &self.state
    }

    fn get_ops(&self) -> Vec<&PcodeOperation> {
        let mut result = vec![];
        for x in self.instr.ops.iter() {
            result.push(x);
        }
        result
    }

    fn get_inputs(&self) -> HashSet<ResolvedVarnode<'ctx>> {
        self.inputs.clone()
    }

    fn get_outputs(&self) -> HashSet<ResolvedVarnode<'ctx>> {
        self.outputs.clone()
    }

    fn get_branch_constraint(&self) -> &BranchConstraint {
        &self.branch_builder
    }
}

impl<'ctx> TranslationContext<'ctx> for ModeledInstruction<'ctx> {
    fn track_input<'a, 'b: 'ctx>(&mut self, input: &'a ResolvedVarnode<'ctx>) {
        self.inputs.insert(input.clone());
    }
    fn track_output(&mut self, output: &ResolvedVarnode<'ctx>) {
        self.outputs.insert(output.clone());
    }

    fn get_final_state_mut(&mut self) -> &mut State<'ctx> {
        &mut self.state
    }

    fn get_branch_builder(&mut self) -> &mut BranchConstraint {
        &mut self.branch_builder
    }
}

/*impl<'ctx> From<&[ModeledInstruction<'ctx>]> for ModeledInstruction<'ctx>{
    fn from(value: &[ModeledInstruction<'ctx>]) -> Self {
        for instr in value.iter() {
            instr.
        }
    }
}*/
