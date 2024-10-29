use crate::modeling::State;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use jingle_sleigh::{SpaceInfo, SpaceManager};
use z3::Context;

#[derive(Debug)]
pub struct JingleContext<'ctx, 'sl> {
    pub z3: &'ctx Context,
    pub sleigh: LoadedSleighContext<'sl>,
}

impl<'ctx, 'a> JingleContext<'ctx, 'a> {
    pub fn new(z3: &'ctx Context, sleigh: LoadedSleighContext<'a>) -> Self {
        Self { z3, sleigh }
    }
    pub fn fresh_state(&self) -> State<'ctx> {
        State::new(self.z3, self)
    }
}

impl<'ctx, 'a> SpaceManager for JingleContext<'ctx, 'a> {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.sleigh.get_space_info(idx)
    }

    fn get_all_space_info(&self) -> &[SpaceInfo] {
        self.sleigh.get_all_space_info()
    }

    fn get_code_space_idx(&self) -> usize {
        self.sleigh.get_code_space_idx()
    }
}
