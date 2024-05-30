use std::collections::HashMap;
use z3::Context;
use jingle_sleigh::{RegisterManager, SpaceInfo, SpaceManager, VarNode};
use crate::modeling::State;

#[derive(Clone, Debug)]
pub struct JingleContext<'ctx> {
    pub z3: &'ctx Context,
    spaces: Vec<SpaceInfo>,
    default_code_space_index: usize,
    varnode_name_mapping: HashMap<VarNode, String>,
    name_varnode_mapping: HashMap<String, VarNode>,
}


impl<'ctx> JingleContext<'ctx> {
    pub fn new<R: RegisterManager>(z3: &'ctx Context, r: &R) -> Self {
        let regs = r.get_registers();
        let mut varnode_name_mapping = HashMap::new();
        let mut name_varnode_mapping = HashMap::new();
        for (vn, s) in regs {
            varnode_name_mapping.insert(vn.clone(), s.clone());
            name_varnode_mapping.insert(s, vn);
        }
        let spaces = r.get_all_space_info().to_vec();
        let default_code_space_index = r.get_code_space_idx();
        Self {
            z3,
            varnode_name_mapping,
            name_varnode_mapping,
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

impl<'ctx> RegisterManager for JingleContext<'ctx> {
    fn get_register(&self, name: &str) -> Option<VarNode> {
        self.name_varnode_mapping.get(name).cloned()
    }

    fn get_register_name(&self, location: VarNode) -> Option<&str> {
        self.varnode_name_mapping.get(&location).map(|f| f.as_str())
    }

    fn get_registers(&self) -> Vec<(VarNode, String)> {
        self.varnode_name_mapping.clone().into_iter().collect()
    }
}
