use crate::modeling::State;
use jingle_sleigh::{ArchInfoProvider, SpaceInfo, VarNode};
use std::ops::Deref;
use std::rc::Rc;
use z3::Context;

#[derive(Clone, Debug)]
pub struct CachedArchInfo {
    registers: Vec<(VarNode, String)>,
    spaces: Vec<SpaceInfo>,
    default_code_space: usize,
}

impl ArchInfoProvider for JingleContext<'_> {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.info.spaces.get(idx)
    }

    fn get_all_space_info(&self) -> impl Iterator<Item = &SpaceInfo> {
        self.info.spaces.iter()
    }

    fn get_code_space_idx(&self) -> usize {
        self.info.default_code_space
    }

    fn get_register(&self, name: &str) -> Option<&VarNode> {
        self.info
            .registers
            .iter()
            .find(|(_, reg_name)| reg_name.as_str() == name)
            .map(|(vn, _)| vn)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.info
            .registers
            .iter()
            .find(|(vn, _)| vn == location)
            .map(|(_, name)| name.as_str())
    }

    fn get_registers(&self) -> impl Iterator<Item = (&VarNode, &str)> {
        self.info.registers.iter().map(|(a,b)| (a,b.as_str()))
    }
}

#[derive(Clone, Debug)]
pub struct JingleContextInternal<'ctx> {
    pub z3: &'ctx Context,
    pub info: CachedArchInfo,
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
    pub fn new<S: ArchInfoProvider>(z3: &'ctx Context, r: &S) -> Self {
        Self(Rc::new(JingleContextInternal {
            z3,
            info: CachedArchInfo {
                spaces: r.get_all_space_info().cloned().collect(),
                registers: r.get_registers().map(|(a,b)|(a.clone(),b.to_string())).collect(),
                default_code_space: r.get_code_space_idx(),
            },
        }))
    }
    pub fn fresh_state(&self) -> State<'ctx> {
        State::new(self)
    }

    pub fn with_fresh_z3_context(&self, z3: &'ctx Context) -> Self {
        Self(Rc::new(JingleContextInternal {
            z3,
            info: self.info.clone(),
        }))
    }
}
