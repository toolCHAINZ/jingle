use crate::JingleContext;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use jingle_sleigh::{ArchInfoProvider, RegisterManager, SpaceInfo, SpaceManager, VarNode};
use std::ops::Deref;
use std::rc::Rc;
use z3::Context;

#[derive(Clone, Debug)]
pub struct CachedArchInfo {
    registers: Vec<(VarNode, String)>,
    spaces: Vec<SpaceInfo>,
    default_code_space: usize,
}

impl ArchInfoProvider for BMCJingleContext<'_> {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.info.spaces.get(idx)
    }

    fn get_all_space_info(&self) -> impl Iterator<Item = &SpaceInfo> {
        self.info.spaces.iter()
    }

    fn get_code_space_idx(&self) -> usize {
        self.info.default_code_space
    }

    fn get_register(&self, name: &str) -> Option<VarNode> {
        self.info
            .registers
            .iter()
            .find(|(_, reg_name)| reg_name.as_str() == name)
            .map(|(vn, _)| vn.clone())
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.info
            .registers
            .iter()
            .find(|(vn, _)| vn == location)
            .map(|(_, name)| name.as_str())
    }

    fn get_registers(&self) -> impl Iterator<Item = &(VarNode, String)> {
        self.info.registers.iter()
    }
}

#[derive(Debug)]
pub struct BMCJingleContextInternal<'ctx> {
    pub z3: &'ctx Context,
    pub info: CachedArchInfo,
}

#[derive(Debug, Clone)]
pub struct BMCJingleContext<'ctx>(Rc<BMCJingleContextInternal<'ctx>>);

impl<'ctx> Deref for BMCJingleContext<'ctx> {
    type Target = BMCJingleContextInternal<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'ctx> BMCJingleContext<'ctx> {
    pub fn new<T: ArchInfoProvider>(z3: &'ctx Context, sleigh: T) -> Self {
        Self(Rc::new(BMCJingleContextInternal {
            z3,
            info: CachedArchInfo {
                spaces: sleigh.get_all_space_info().cloned().collect(),
                registers: sleigh.get_registers().cloned().collect(),
                default_code_space: sleigh.get_code_space_idx(),
            },
        }))
    }
    pub fn with_fresh_z3_context<'ctx2>(&self, z3: &'ctx2 Context) -> BMCJingleContext<'ctx2> {
        BMCJingleContext(Rc::new(BMCJingleContextInternal {
            z3,
            info: self.info.clone(),
        }))
    }
}
