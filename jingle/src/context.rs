use crate::modeling::State;
use jingle_sleigh::{RegisterManager, SpaceInfo, SpaceManager, VarNode};
use std::ops::Deref;
use std::rc::Rc;
use z3::Context;

#[derive(Clone, Debug)]
pub struct JingleContextInternal<'ctx> {
    pub z3: &'ctx Context,
    spaces: Vec<SpaceInfo>,
    default_code_space_index: usize,
    registers: Vec<(VarNode, String)>,
}

#[derive(Clone, Debug)]
pub struct JingleContext<'ctx>(Rc<JingleContextInternal<'ctx>>);

impl<'ctx> Deref for JingleContext<'ctx> {
    type Target = JingleContextInternal<'ctx>;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl<'ctx> JingleContext<'ctx> {
    pub fn new<S: RegisterManager>(z3: &'ctx Context, r: &S) -> Self {
        let spaces = r.get_all_space_info().to_vec();
        let default_code_space_index = r.get_code_space_idx();
        Self(Rc::new(JingleContextInternal {
            z3,
            spaces,
            default_code_space_index,
            registers: r.get_registers(),
        }))
    }
    pub fn fresh_state(&self) -> State<'ctx> {
        State::new(self)
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

impl<'ctx> RegisterManager for JingleContext<'ctx> {
    fn get_register(&self, name: &str) -> Option<VarNode> {
        self.registers
            .iter()
            .find_map(|i| i.1.eq(name).then_some(i.0.clone()))
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.registers
            .iter()
            .find_map(|i| i.0.eq(location).then_some(i.1.as_str()))
    }

    fn get_registers(&self) -> Vec<(VarNode, String)> {
        self.registers.clone()
    }
}
