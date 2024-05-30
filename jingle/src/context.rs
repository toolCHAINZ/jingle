use z3::Context;
use jingle_sleigh::SpaceInfo;
use crate::modeling::State;

struct SleighModelInfo {
    spaces: Vec<SpaceInfo>,
    default_code_space_index: usize
}
pub(crate) struct JingleContext<'ctx>{
    z3: &'ctx Context,
    sleigh_model_info: SleighModelInfo
}


impl JingleContext {
    pub fn fresh_state(&self) -> State{
        State::new(self.z3, &self)
    }
}

impl From<&JingleContext> for &Context{
    fn from(value: &JingleContext) -> Self {
        value.z3
    }
}