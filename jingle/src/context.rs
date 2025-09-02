use crate::modeling::State;
use jingle_sleigh::{ArchInfoProvider, SleighArchInfo, SpaceInfo, VarNode};
use std::ops::Deref;
use std::rc::Rc;

impl ArchInfoProvider for JingleContext {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.info.get_space_info(idx)
    }

    fn get_all_space_info(&self) -> impl Iterator<Item = &SpaceInfo> {
        self.info.get_all_space_info()
    }

    fn get_code_space_idx(&self) -> usize {
        self.info.get_code_space_idx()
    }

    fn get_register(&self, name: &str) -> Option<&VarNode> {
        self.info.get_register(name)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.info.get_register_name(location)
    }

    fn get_registers(&self) -> impl Iterator<Item = (&VarNode, &str)> {
        self.info.get_registers()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JingleContextInternal {
    pub info: SleighArchInfo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JingleContext(Rc<JingleContextInternal>);

impl Deref for JingleContext {
    type Target = JingleContextInternal;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl JingleContext {
    pub fn new<S: ArchInfoProvider>(r: &S) -> Self {
        Self(Rc::new(JingleContextInternal {
            info: SleighArchInfo::new(
                r.get_registers(),
                r.get_all_space_info(),
                r.get_code_space_idx(),
            ),
        }))
    }

    pub fn fresh_state(&self) -> State {
        State::new(self)
    }
}
