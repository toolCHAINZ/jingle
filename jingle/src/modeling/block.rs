use crate::error::JingleError;
use crate::error::JingleError::DisassemblyLengthBound;
use crate::modeling::branch::BranchConstraint;
use crate::modeling::state::State;
use crate::modeling::{ModelingContext, TranslationContext};
use crate::varnode::ResolvedVarnode;
use crate::JingleContext;
use crate::JingleError::EmptyBlock;
use jingle_sleigh::PcodeOperation;
use jingle_sleigh::SpaceInfo;
use jingle_sleigh::{ArchInfoProvider, Instruction, VarNode};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};

/// A `jingle` model of a basic block
#[derive(Debug, Clone)]
pub struct ModeledBlock<'ctx> {
    jingle: JingleContext<'ctx>,
    pub instructions: Vec<Instruction>,
    state: State<'ctx>,
    original_state: State<'ctx>,
    branch_constraint: BranchConstraint,
    inputs: HashSet<ResolvedVarnode<'ctx>>,
    outputs: HashSet<ResolvedVarnode<'ctx>>,
}

impl Display for ModeledBlock<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for x in self.instructions.iter() {
            writeln!(f, "{:x} {}", x.address, x.disassembly)?;
        }
        Ok(())
    }
}

impl<'ctx, T: ModelingContext<'ctx>> TryFrom<&'ctx [T]> for ModeledBlock<'ctx> {
    type Error = JingleError;
    fn try_from(vec: &'ctx [T]) -> Result<Self, Self::Error> {
        let jingle = vec.first().ok_or(EmptyBlock)?.get_jingle();
        let original_state = State::new(jingle);
        let state = original_state.clone();
        let mut new_block: Self = Self {
            jingle: jingle.clone(),
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

impl<'ctx> ModeledBlock<'ctx> {
    pub fn read<T: Iterator<Item = Instruction>>(
        jingle: &JingleContext<'ctx>,
        instr_iter: T,
    ) -> Result<Self, JingleError> {
        let original_state = State::new(jingle);
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
            jingle: jingle.clone(),
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
        ModeledBlock::read(&self.jingle, self.instructions.clone().into_iter())
    }

    pub fn get_first_address(&self) -> u64 {
        self.instructions[0].address
    }

    pub fn get_last_address(&self) -> u64 {
        let i = self.instructions.last().unwrap();
        i.address + i.length as u64
    }
}

impl ArchInfoProvider for ModeledBlock<'_> {
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

impl<'ctx> ModelingContext<'ctx> for ModeledBlock<'ctx> {
    fn get_jingle(&self) -> &JingleContext<'ctx> {
        &self.jingle
    }

    fn get_address(&self) -> u64 {
        self.instructions[0].address
    }

    fn get_original_state(&self) -> &State<'ctx> {
        &self.original_state
    }

    fn get_final_state(&self) -> &State<'ctx> {
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

    fn get_inputs(&self) -> HashSet<ResolvedVarnode<'ctx>> {
        self.inputs.clone()
    }

    fn get_outputs(&self) -> HashSet<ResolvedVarnode<'ctx>> {
        self.outputs.clone()
    }

    fn get_branch_constraint(&self) -> &BranchConstraint {
        &self.branch_constraint
    }
}

impl<'ctx> TranslationContext<'ctx> for ModeledBlock<'ctx> {
    fn track_input<'a, 'b: 'ctx>(&mut self, input: &ResolvedVarnode<'ctx>) {
        self.inputs.insert(input.clone());
    }
    fn track_output(&mut self, output: &ResolvedVarnode<'ctx>) {
        self.outputs.insert(output.clone());
    }

    fn get_final_state_mut(&mut self) -> &mut State<'ctx> {
        &mut self.state
    }

    fn get_branch_builder(&mut self) -> &mut BranchConstraint {
        &mut self.branch_constraint
    }
}
