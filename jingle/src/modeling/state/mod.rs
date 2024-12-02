mod space;

use crate::error::JingleError;
use crate::error::JingleError::{
    ConstantWrite, IndirectConstantRead, MismatchedWordSize, UnexpectedArraySort, UnmodeledSpace,
    ZeroSizedVarnode,
};

use crate::modeling::state::space::ModeledSpace;
use crate::varnode::ResolvedVarnode;
use crate::JingleContext;
use jingle_sleigh::{
    GeneralizedVarNode, IndirectVarNode, RegisterManager, SpaceInfo, SpaceManager, SpaceType,
    VarNode,
};
use std::ops::Add;
use z3::ast::{Array, Ast, Bool, BV};

/// Represents the modeled combined memory state of the system. State
/// is represented with Z3 formulas built up as select and store operations
/// on an initial state
#[derive(Clone, Debug)]
pub struct State<'ctx> {
    jingle: JingleContext<'ctx>,
    spaces: Vec<ModeledSpace<'ctx>>,
}

impl SpaceManager for State<'_> {
    fn get_space_info(&self, idx: usize) -> Option<&SpaceInfo> {
        self.jingle.get_space_info(idx)
    }

    fn get_all_space_info(&self) -> &[SpaceInfo] {
        self.jingle.get_all_space_info()
    }

    fn get_code_space_idx(&self) -> usize {
        self.jingle.get_code_space_idx()
    }
}

impl RegisterManager for State<'_> {
    fn get_register(&self, name: &str) -> Option<VarNode> {
        self.jingle.get_register(name)
    }

    fn get_register_name(&self, location: &VarNode) -> Option<&str> {
        self.jingle.get_register_name(location)
    }

    fn get_registers(&self) -> Vec<(VarNode, String)> {
        self.jingle.get_registers()
    }
}

impl<'ctx> State<'ctx> {
    pub fn new(jingle: &JingleContext<'ctx>) -> Self {
        let mut spaces: Vec<ModeledSpace> = Default::default();
        for space_info in jingle.get_all_space_info() {
            spaces.push(ModeledSpace::new(jingle, space_info));
        }
        Self {
            jingle: jingle.clone(),
            spaces,
        }
    }

    pub fn get_space(&self, idx: usize) -> Result<&Array<'ctx>, JingleError> {
        self.spaces
            .get(idx)
            .map(|u| u.get_space())
            .ok_or(UnmodeledSpace)
    }

    pub fn read_varnode<'a>(&'a self, varnode: &VarNode) -> Result<BV<'ctx>, JingleError> {
        let space = self
            .get_space_info(varnode.space_index)
            .ok_or(UnmodeledSpace)?;
        match space._type {
            SpaceType::IPTR_CONSTANT => Ok(BV::from_i64(
                self.jingle.z3,
                varnode.offset as i64,
                (varnode.size * 8) as u32,
            )),
            _ => {
                let offset = BV::from_i64(
                    self.jingle.z3,
                    varnode.offset as i64,
                    space.index_size_bytes * 8,
                );
                let arr = self.spaces.get(varnode.space_index).ok_or(UnmodeledSpace)?;
                arr.read_data(&offset, varnode.size)
            }
        }
    }

    pub fn read_varnode_metadata<'a>(&'a self, varnode: &VarNode) -> Result<BV<'ctx>, JingleError> {
        let space = self
            .get_space_info(varnode.space_index)
            .ok_or(UnmodeledSpace)?;

        let offset = BV::from_i64(
            self.jingle.z3,
            varnode.offset as i64,
            space.index_size_bytes * 8,
        );
        let arr = self.spaces.get(varnode.space_index).ok_or(UnmodeledSpace)?;
        arr.read_metadata(&offset, varnode.size)
    }

    pub fn read_varnode_indirect<'a, 'b: 'ctx>(
        &'a self,
        indirect: &IndirectVarNode,
    ) -> Result<BV<'ctx>, JingleError> {
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

    pub fn read_varnode_metadata_indirect<'a, 'b: 'ctx>(
        &'a self,
        indirect: &IndirectVarNode,
    ) -> Result<BV<'ctx>, JingleError> {
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

    pub fn read(&self, vn: GeneralizedVarNode) -> Result<BV<'ctx>, JingleError> {
        match vn {
            GeneralizedVarNode::Direct(d) => self.read_varnode(&d),
            GeneralizedVarNode::Indirect(i) => self.read_varnode_indirect(&i),
        }
    }

    pub fn read_metadata(&self, vn: GeneralizedVarNode) -> Result<BV<'ctx>, JingleError> {
        match vn {
            GeneralizedVarNode::Direct(d) => self.read_varnode_metadata(&d),
            GeneralizedVarNode::Indirect(i) => self.read_varnode_metadata_indirect(&i),
        }
    }

    /// Model a write to a [VarNode] on top of the current context.
    pub fn write_varnode<'a, 'b: 'ctx>(
        &'a mut self,
        dest: &VarNode,
        val: BV<'b>,
    ) -> Result<(), JingleError> {
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
                space.write_data(
                    &val,
                    &BV::from_u64(self.jingle.z3, dest.offset, info.index_size_bytes * 8),
                )?;
                Ok(())
            }
        }
    }

    pub fn write_varnode_metadata<'a, 'b: 'ctx>(
        &'a mut self,
        dest: &VarNode,
        val: BV<'b>,
    ) -> Result<(), JingleError> {
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

        space.write_metadata(
            &val,
            &BV::from_u64(self.jingle.z3, dest.offset, info.index_size_bytes * 8),
        )?;
        Ok(())
    }

    /// Model a write to an [IndirectVarNode] on top of the current context.
    pub fn write_varnode_indirect<'a>(
        &'a mut self,
        dest: &IndirectVarNode,
        val: BV<'ctx>,
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

    pub fn write_varnode_metadata_indirect<'a>(
        &'a mut self,
        dest: &IndirectVarNode,
        val: BV<'ctx>,
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

    pub fn read_resolved<'a, 'b: 'ctx, 'c>(
        &'a self,
        vn: &'a ResolvedVarnode<'b>,
    ) -> Result<BV<'ctx>, JingleError> {
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

    pub fn get_default_code_space(&self) -> &Array<'ctx> {
        self.spaces[self.jingle.get_code_space_idx()].get_space()
    }

    pub fn get_default_code_space_info(&self) -> &SpaceInfo {
        self.jingle
            .get_space_info(self.jingle.get_code_space_idx())
            .unwrap()
    }

    pub(crate) fn immediate_metadata_array(&self, val: bool, s: usize) -> BV<'ctx> {
        let val = match val {
            true => 1,
            false => 0,
        };
        (0..s)
            .map(|_| BV::from_u64(self.jingle.z3, val, 1))
            .reduce(|a, b| a.concat(&b))
            .map(|b| b.simplify())
            .unwrap()
    }

    pub fn _eq(&self, other: &State<'ctx>) -> Result<Bool<'ctx>, JingleError> {
        let mut terms = vec![];
        for (i, _) in self
            .get_all_space_info()
            .iter()
            .enumerate()
            .filter(|(_, n)| n._type == SpaceType::IPTR_PROCESSOR)
        {
            let self_space = self.get_space(i)?;
            let other_space = other.get_space(i)?;
            terms.push(self_space._eq(other_space))
        }
        let eq_terms: Vec<&Bool> = terms.iter().collect();
        Ok(Bool::and(self.jingle.z3, eq_terms.as_slice()))
    }
}
