use crate::modeling::bmc::memory_state::MemoryState;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use jingle_sleigh::{SpaceInfo, SpaceManager};
use z3::Context;

#[derive(Debug)]
pub struct BMCJingleContext<'ctx, 'sl> {
    pub z3: &'ctx Context,
    pub sleigh: LoadedSleighContext<'sl>,
}

impl<'ctx, 'sl> BMCJingleContext<'ctx, 'sl> {
    pub fn new(z3: &'ctx Context, sleigh: LoadedSleighContext<'sl>) -> Self {
        Self { z3, sleigh }
    }
    pub fn fresh_state<'b>(&'b self) -> MemoryState<'b, 'ctx, 'sl> {
        MemoryState::new(self)
    }
}
impl SpaceManager for BMCJingleContext<'_, '_> {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.sleigh.get_space_info(idx)
    }

    fn get_all_space_info(&self) -> &[SpaceInfo] {
        todo!()
    }

    fn get_code_space_idx(&self) -> usize {
        todo!()
    }
}
