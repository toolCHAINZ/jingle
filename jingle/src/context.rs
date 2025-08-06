use crate::modeling::State;
use jingle_sleigh::{ArchInfoProvider, SpaceInfo, VarNode};
use std::ops::Deref;
use std::rc::Rc;
use z3::Context;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedArchInfo {
    registers: Vec<(VarNode, String)>,
    spaces: Vec<SpaceInfo>,
    default_code_space: usize,
}

impl ArchInfoProvider for JingleContext {
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
        self.info.registers.iter().map(|(a, b)| (a, b.as_str()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JingleContextInternal {
    pub z3: Context,
    pub info: CachedArchInfo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JingleContext(Rc<JingleContextInternal>);

impl JingleContext {
    pub(crate) fn translate(&self, ctx: &Context) -> JingleContext {
        JingleContext(Rc::new(JingleContextInternal {
            z3: ctx.clone(),
            info: self.info.clone(),
        }))
    }
}

impl Deref for JingleContext {
    type Target = JingleContextInternal;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl JingleContext {
    pub fn new<S: ArchInfoProvider>(z3: &Context, r: &S) -> Self {
        Self(Rc::new(JingleContextInternal {
            z3: z3.clone(),
            info: CachedArchInfo {
                spaces: r.get_all_space_info().cloned().collect(),
                registers: r
                    .get_registers()
                    .map(|(a, b)| (a.clone(), b.to_string()))
                    .collect(),
                default_code_space: r.get_code_space_idx(),
            },
        }))
    }
    
    pub fn ctx(&self) -> &Context {
        &self.z3
    }
    pub fn fresh_state(&self) -> State {
        State::new(self)
    }

    pub fn with_fresh_z3_context(&self, z3: & Context) -> Self {
        Self(Rc::new(JingleContextInternal {
            z3: z3.clone(),
            info: self.info.clone(),
        }))
    }
}
