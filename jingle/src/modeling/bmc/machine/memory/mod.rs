mod relations;
pub mod space;

use crate::modeling::bmc::context::BMCJingleContext;
use crate::modeling::bmc::machine::memory::space::BMCModeledSpace;
use crate::varnode::ResolvedVarnode;
use crate::JingleError;
use crate::JingleError::{
    ConstantWrite, IndirectConstantRead, MismatchedWordSize, UnexpectedArraySort, UnmodeledSpace,
    ZeroSizedVarnode,
};
use jingle_sleigh::{
    GeneralizedVarNode, IndirectVarNode, SpaceInfo, SpaceManager, SpaceType, VarNode,
};
use std::ops::Add;
use z3::ast::{Array, Ast, Bool, BV};

/// Represents the modeled combined memory state of the system. State
/// is represented with Z3 formulas built up as select and store operations
/// on an initial state
#[derive(Clone, Debug)]
pub struct MemoryState<'ctx, 'sl> {
    jingle: BMCJingleContext<'ctx, 'sl>,
    spaces: Vec<BMCModeledSpace<'ctx>>,
}

impl<'ctx, 'sl> SpaceManager for MemoryState<'ctx, 'sl> {
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

impl<'ctx, 'sl> MemoryState<'ctx, 'sl> {
    pub fn fresh(jingle: &BMCJingleContext<'ctx, 'sl>) -> Self {
        let jingle = jingle.clone();
        let spaces: Vec<BMCModeledSpace<'ctx>> = jingle
            .get_all_space_info()
            .iter()
            .map(|s| BMCModeledSpace::new(jingle.z3, s))
            .collect();
        Self { jingle, spaces }
    }

    pub fn get_space(&self, idx: usize) -> Result<&Array<'ctx>, JingleError> {
        self.spaces
            .get(idx)
            .map(|u| u.get_space())
            .ok_or(UnmodeledSpace)
    }

    fn read_varnode(&self, varnode: &VarNode) -> Result<BV<'ctx>, JingleError> {
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
                arr.read(&offset, varnode.size)
            }
        }
    }

    fn read_varnode_indirect(&self, indirect: &IndirectVarNode) -> Result<BV<'ctx>, JingleError> {
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
        space.read(&ptr, indirect.access_size_bytes)
    }

    fn read_varnode_metadata_indirect(
        &self,
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
        space.read(&ptr, indirect.access_size_bytes)
    }

    pub fn read<T: Into<GeneralizedVarNode>>(&self, vn: T) -> Result<BV<'ctx>, JingleError> {
        let gen: GeneralizedVarNode = vn.into();
        match gen {
            GeneralizedVarNode::Direct(d) => self.read_varnode(&d),
            GeneralizedVarNode::Indirect(i) => self.read_varnode_indirect(&i),
        }
    }

    pub fn write<T: Into<GeneralizedVarNode>>(
        self,
        dest: T,
        val: BV<'ctx>,
    ) -> Result<Self, JingleError> {
        let gen: GeneralizedVarNode = dest.into();
        match gen {
            GeneralizedVarNode::Direct(d) => self.write_varnode(&d, val),
            GeneralizedVarNode::Indirect(i) => self.write_varnode_indirect(&i, val),
        }
    }

    /// Model a write to a [VarNode] on top of the current context.
    fn write_varnode(mut self, dest: &VarNode, val: BV<'ctx>) -> Result<Self, JingleError> {
        if dest.size as u32 * 8 != val.get_size() {
            return Err(MismatchedWordSize);
        }
        match self.jingle.get_space_info(dest.space_index).unwrap() {
            SpaceInfo {
                _type: SpaceType::IPTR_CONSTANT,
                ..
            } => Err(ConstantWrite),
            info => {
                let space = self
                    .spaces
                    .get_mut(dest.space_index)
                    .ok_or(UnmodeledSpace)?;
                space.write(
                    &val,
                    &BV::from_u64(self.jingle.z3, dest.offset, info.index_size_bytes * 8),
                )?;
                Ok(self)
            }
        }
    }

    /// Model a write to an [IndirectVarNode] on top of the current context.
    fn write_varnode_indirect(
        mut self,
        dest: &IndirectVarNode,
        val: BV<'ctx>,
    ) -> Result<Self, JingleError> {
        if self.get_space_info(dest.pointer_space_index).unwrap()._type == SpaceType::IPTR_CONSTANT
        {
            return Err(ConstantWrite);
        }
        let ptr = self.read_varnode(&dest.pointer_location)?;
        self.spaces[dest.pointer_space_index].write(&val, &ptr)?;
        Ok(self)
    }

    fn write_varnode_metadata_indirect(
        &mut self,
        dest: &IndirectVarNode,
        val: BV<'ctx>,
    ) -> Result<(), JingleError> {
        if self.get_space_info(dest.pointer_space_index).unwrap()._type == SpaceType::IPTR_CONSTANT
        {
            return Err(ConstantWrite);
        }
        let ptr = self.read_varnode(&dest.pointer_location)?;
        self.spaces[dest.pointer_space_index].write(&val, &ptr)?;
        Ok(())
    }

    pub fn read_resolved(&self, vn: &ResolvedVarnode<'ctx>) -> Result<BV<'ctx>, JingleError> {
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

    pub fn _eq(&self, other: &MemoryState<'ctx, '_>) -> Result<Bool<'ctx>, JingleError> {
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
