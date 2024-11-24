use jingle_sleigh::context::loaded::LoadedSleighContext;
use jingle_sleigh::{SpaceInfo, SpaceManager};
use std::rc::Rc;
use z3::Context;

#[derive(Debug, Clone)]
pub struct BMCJingleContext<'ctx, 'sl> {
    pub z3: &'ctx Context,
    pub sleigh: Rc<LoadedSleighContext<'sl>>,
}

impl<'ctx, 'sl> BMCJingleContext<'ctx, 'sl> {
    pub fn new(z3: &'ctx Context, sleigh: LoadedSleighContext<'sl>) -> Self {
        Self {
            z3,
            sleigh: Rc::new(sleigh),
        }
    }
    pub fn with_fresh_z3_context<'ctx2>(&self, z3: &'ctx2 Context) -> BMCJingleContext<'ctx2, 'sl> {
        BMCJingleContext{
            z3,
            sleigh: self.sleigh.clone()
        }
    }
}
impl SpaceManager for BMCJingleContext<'_, '_> {
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
