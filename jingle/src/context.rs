use crate::modeling::State;
use jingle_sleigh::{SpaceInfo, SpaceManager};
use z3::Context;

#[derive(Clone, Debug)]
pub struct JingleContext<'ctx> {
    pub z3: &'ctx Context,
    spaces: Vec<SpaceInfo>,
    default_code_space_index: usize,
}

impl<'ctx> JingleContext<'ctx> {
    pub fn new<S: SpaceManager>(z3: &'ctx Context, r: &S) -> Self {
        let spaces = r.get_all_space_info().to_vec();
        let default_code_space_index = r.get_code_space_idx();
        Self {
            z3,
            spaces,
            default_code_space_index,
        }
    }
    pub fn fresh_state(&self) -> State<'ctx> {
        State::new(self.z3, self)
    }
}

impl<'ctx> SpaceManager for JingleContext<'ctx> {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.spaces.get(idx)
    }

    fn get_all_space_info(&self) -> &[SpaceInfo] {
        self.spaces.as_slice()
    }

    fn get_code_space_idx(&self) -> usize {
        self.default_code_space_index
    }
}
