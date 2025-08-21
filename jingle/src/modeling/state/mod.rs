mod space;

use crate::error::JingleError;
use crate::error::JingleError::{
    ConstantWrite, IndirectConstantRead, MismatchedWordSize, UnexpectedArraySort, UnmodeledSpace,
    ZeroSizedVarnode,
};

use crate::JingleContext;
use crate::modeling::state::space::ModeledSpace;
use crate::varnode::ResolvedVarnode;
use jingle_sleigh::{
    ArchInfoProvider, GeneralizedVarNode, IndirectVarNode, SpaceInfo, SpaceType, VarNode,
};
use std::ops::Add;
use z3::ast::{Array, Ast, BV, Bool};
use z3::{Context, Translate};

/// Represents the modeled combined memory state of the system. State
/// is represented with Z3 formulas built up as select and store operations
/// on an initial state
#[derive(Clone, Debug)]
pub struct State {
    jingle: JingleContext,
    spaces: Vec<ModeledSpace>,
}

impl ArchInfoProvider for State {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.jingle.get_space_info(idx)
    }

    fn get_all_space_info(&self) -> impl Iterator<Item = &SpaceInfo> {
        self.jingle.get_all_space_info()
    }
    fn get_code_space_idx(&self) -> usize {
        self.jingle.get_code_space_idx()
    }

    fn get_register(&self, name: &str) -> Option<&VarNode> {
        self.jingle.get_register(name)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.jingle.get_register_name(location)
    }

    fn get_registers(&self) -> impl Iterator<Item = (&VarNode, &str)> {
        self.jingle.get_registers()
    }
}

impl State {
    pub fn new(jingle: &JingleContext) -> Self {
        let mut spaces: Vec<ModeledSpace> = Default::default();
        for space_info in jingle.get_all_space_info() {
            spaces.push(ModeledSpace::new(space_info));
        }
        Self {
            jingle: jingle.clone(),
            spaces,
        }
    }

    pub fn get_space(&self, idx: usize) -> Result<&Array, JingleError> {
        self.spaces
            .get(idx)
            .map(|u| u.get_space())
            .ok_or(UnmodeledSpace)
    }

    pub fn read_varnode(&self, varnode: &VarNode) -> Result<BV, JingleError> {
        let space = self
            .get_space_info(varnode.space_index)
            .ok_or(UnmodeledSpace)?;
        match space._type {
            SpaceType::IPTR_CONSTANT => Ok(BV::from_i64(
                varnode.offset as i64,
                (varnode.size * 8) as u32,
            )),
            _ => {
                let offset = BV::from_i64(varnode.offset as i64, space.index_size_bytes * 8);
                let arr = self.spaces.get(varnode.space_index).ok_or(UnmodeledSpace)?;
                arr.read_data(&offset, varnode.size)
            }
        }
    }

    pub fn read_varnode_metadata(&self, varnode: &VarNode) -> Result<BV, JingleError> {
        let space = self
            .get_space_info(varnode.space_index)
            .ok_or(UnmodeledSpace)?;

        let offset = BV::from_i64(varnode.offset as i64, space.index_size_bytes * 8);
        let arr = self.spaces.get(varnode.space_index).ok_or(UnmodeledSpace)?;
        arr.read_metadata(&offset, varnode.size)
    }

    pub fn read_varnode_indirect(&self, indirect: &IndirectVarNode) -> Result<BV, JingleError> {
        let pointer_space_info = self
            .get_space_info(indirect.pointer_space_index)
            .ok_or(UnmodeledSpace)?;
        if pointer_space_info._type == SpaceType::IPTR_CONSTANT {
            return Err(IndirectConstantRead);
        }
        let ptr = self.read_varnode(&indirect.pointer_location)?;

        let space = self
            .spaces
            .get(indirect.pointer_space_index)
            .ok_or(UnmodeledSpace)?;
        space.read_data(&ptr, indirect.access_size_bytes)
    }

    pub fn read_varnode_metadata_indirect(
        &self,
        indirect: &IndirectVarNode,
    ) -> Result<BV, JingleError> {
        let pointer_space_info = self
            .get_space_info(indirect.pointer_space_index)
            .ok_or(UnmodeledSpace)?;
        if pointer_space_info._type == SpaceType::IPTR_CONSTANT {
            return Err(IndirectConstantRead);
        }
        let ptr = self.read_varnode(&indirect.pointer_location)?;

        let space = self
            .spaces
            .get(indirect.pointer_space_index)
            .ok_or(UnmodeledSpace)?;
        space.read_metadata(&ptr, indirect.access_size_bytes)
    }

    pub fn read(&self, vn: GeneralizedVarNode) -> Result<BV, JingleError> {
        match vn {
            GeneralizedVarNode::Direct(d) => self.read_varnode(&d),
            GeneralizedVarNode::Indirect(i) => self.read_varnode_indirect(&i),
        }
    }

    pub fn read_metadata(&self, vn: GeneralizedVarNode) -> Result<BV, JingleError> {
        match vn {
            GeneralizedVarNode::Direct(d) => self.read_varnode_metadata(&d),
            GeneralizedVarNode::Indirect(i) => self.read_varnode_metadata_indirect(&i),
        }
    }

    /// Model a write to a [VarNode] on top of the current context.
    pub fn write_varnode(&mut self, dest: &VarNode, val: BV) -> Result<(), JingleError> {
        if dest.size as u32 * 8 != val.get_size() {
            return Err(MismatchedWordSize);
        }
        let info = self
            .jingle
            .get_space_info(dest.space_index)
            .ok_or(UnmodeledSpace)?;
        match info._type {
            SpaceType::IPTR_CONSTANT => Err(ConstantWrite),
            _ => {
                let space = self
                    .spaces
                    .get_mut(dest.space_index)
                    .ok_or(UnmodeledSpace)?;
                space.write_data(&val, &BV::from_u64(dest.offset, info.index_size_bytes * 8))?;
                Ok(())
            }
        }
    }

    pub fn write_varnode_metadata(&mut self, dest: &VarNode, val: BV) -> Result<(), JingleError> {
        if dest.size != val.get_size() as usize {
            return Err(MismatchedWordSize);
        }
        // We are allowing writes to the constant space for metadata
        // to allow flagging userop values for syscalls
        let space = self
            .spaces
            .get_mut(dest.space_index)
            .ok_or(UnmodeledSpace)?;
        let info = self
            .jingle
            .get_space_info(dest.space_index)
            .ok_or(UnmodeledSpace)?;

        space.write_metadata(&val, &BV::from_u64(dest.offset, info.index_size_bytes * 8))?;
        Ok(())
    }

    /// Model a write to an [IndirectVarNode] on top of the current context.
    pub fn write_varnode_indirect(
        &mut self,
        dest: &IndirectVarNode,
        val: BV,
    ) -> Result<(), JingleError> {
        let info = self
            .jingle
            .get_space_info(dest.pointer_space_index)
            .ok_or(UnmodeledSpace)?;

        if info._type == SpaceType::IPTR_CONSTANT {
            return Err(ConstantWrite);
        }
        let ptr = self.read_varnode(&dest.pointer_location)?;
        self.spaces[dest.pointer_space_index].write_data(&val, &ptr)?;
        Ok(())
    }

    pub fn write_varnode_metadata_indirect(
        &mut self,
        dest: &IndirectVarNode,
        val: BV,
    ) -> Result<(), JingleError> {
        let info = self
            .jingle
            .get_space_info(dest.pointer_space_index)
            .ok_or(UnmodeledSpace)?;

        if info._type == SpaceType::IPTR_CONSTANT {
            return Err(ConstantWrite);
        }
        let ptr = self.read_varnode(&dest.pointer_location)?;
        self.spaces[dest.pointer_space_index].write_metadata(&val, &ptr)?;
        Ok(())
    }

    pub fn read_resolved<'a>(&'a self, vn: &'a ResolvedVarnode) -> Result<BV, JingleError> {
        match vn {
            ResolvedVarnode::Direct(d) => self.read_varnode(d),
            ResolvedVarnode::Indirect(indirect) => {
                let array = self.get_space(indirect.pointer_space_idx)?;
                (0..indirect.access_size_bytes)
                    .map(|i| {
                        array
                            .select(&indirect.pointer.clone().add(i as u64))
                            .as_bv()
                            .ok_or(UnexpectedArraySort)
                    })
                    .reduce(|c, d| Ok(d?.concat(&c?)))
                    .ok_or(ZeroSizedVarnode)?
            }
        }
    }

    pub fn get_default_code_space(&self) -> &Array {
        self.spaces[self.jingle.get_code_space_idx()].get_space()
    }

    pub fn get_default_code_space_info(&self) -> &SpaceInfo {
        self.jingle
            .get_space_info(self.jingle.get_code_space_idx())
            .unwrap()
    }

    pub(crate) fn immediate_metadata_array(&self, val: bool, s: usize) -> BV {
        let val = match val {
            true => 1,
            false => 0,
        };
        (0..s)
            .map(|_| BV::from_u64(val, 1))
            .reduce(|a, b| a.concat(&b))
            .map(|b| b.simplify())
            .unwrap()
    }

    pub fn _eq(&self, other: &State) -> Result<Bool, JingleError> {
        let mut terms = vec![];
        for (i, _) in self
            .get_all_space_info()
            .enumerate()
            .filter(|(_, n)| n._type == SpaceType::IPTR_PROCESSOR)
        {
            let self_space = self.get_space(i)?;
            let other_space = other.get_space(i)?;
            terms.push(self_space._eq(other_space))
        }
        let eq_terms: Vec<&Bool> = terms.iter().collect();
        Ok(Bool::and(eq_terms.as_slice()))
    }

    pub fn fmt_smt_arrays(&self) -> String {
        let mut lines = vec![];
        for x in &self.spaces {
            lines.push(x.fmt_smt_array())
        }
        lines.join("\n")
    }
}

unsafe impl Translate for State {
    fn translate(&self, ctx: &Context) -> Self {
        State {
            spaces: self.spaces.iter().map(|s| s.translate(ctx)).collect(),
            jingle: self.jingle.clone(),
        }
    }
}
