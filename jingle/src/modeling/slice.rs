use crate::modeling::{BranchConstraint, ModelingContext, State};
use crate::varnode::ResolvedVarnode;
use crate::JingleContext;
use jingle_sleigh::PcodeOperation;
use std::collections::HashSet;

impl<'ctx, T: ModelingContext<'ctx>> ModelingContext<'ctx> for &[T] {
    fn get_jingle(&self) -> &JingleContext<'ctx> {
        self[0].get_jingle()
    }

    fn get_address(&self) -> u64 {
        self[0].get_address()
    }

    fn get_original_state(&self) -> &State<'ctx> {
        self[0].get_original_state()
    }

    fn get_final_state(&self) -> &State<'ctx> {
        self.last().unwrap().get_final_state()
    }

    fn get_ops(&self) -> Vec<&PcodeOperation> {
        let mut vec = vec![];
        for thing in self.iter() {
            vec.extend(thing.get_ops())
        }
        vec
    }

    fn get_inputs(&self) -> HashSet<ResolvedVarnode<'ctx>> {
        // todo: this can have some inputs removed if they exist as outputs of a previous thing
        let mut hash_set = HashSet::new();
        for thing in self.iter() {
            hash_set.extend(thing.get_inputs());
        }
        hash_set
    }

    fn get_outputs(&self) -> HashSet<ResolvedVarnode<'ctx>> {
        let mut hash_set = HashSet::new();
        for thing in self.iter() {
            hash_set.extend(thing.get_outputs());
        }
        hash_set
    }

    fn get_branch_constraint(&self) -> &BranchConstraint {
        self.last().unwrap().get_branch_constraint()
    }
}

