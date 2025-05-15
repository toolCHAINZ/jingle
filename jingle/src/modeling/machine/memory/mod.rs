mod relations;
pub mod space;

use crate::JingleError::{
    ConstantWrite, IndirectConstantRead, MismatchedWordSize, UnexpectedArraySort, UnmodeledSpace,
    ZeroSizedVarnode,
};
use crate::modeling::machine::memory::space::BMCModeledSpace;
use crate::varnode::ResolvedVarnode;
use crate::{JingleContext, JingleError};
use jingle_sleigh::{
    ArchInfoProvider, GeneralizedVarNode, IndirectVarNode, SpaceInfo, SpaceType, VarNode,
};
use std::ops::Add;
use z3::ast::{Array, BV, Bool};

/// Represents the modeled combined memory state of the system. State
/// is represented with Z3 formulas built up as select and store operations
/// on an initial state
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryState<'ctx> {
    jingle: JingleContext<'ctx>,
    spaces: Vec<BMCModeledSpace<'ctx>>,
}

impl<'ctx> MemoryState<'ctx> {
    pub fn fresh(jingle: &JingleContext<'ctx>) -> Self {
        let jingle = jingle.clone();
        let spaces: Vec<BMCModeledSpace<'ctx>> = jingle
            .get_all_space_info()
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
            .jingle
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
            .jingle
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

    #[expect(unused)]
    fn read_varnode_metadata_indirect(
        &self,
        indirect: &IndirectVarNode,
    ) -> Result<BV<'ctx>, JingleError> {
        let pointer_space_info = self
            .jingle
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
        let gen_varnode: GeneralizedVarNode = vn.into();
        match gen_varnode {
            GeneralizedVarNode::Direct(d) => self.read_varnode(&d),
            GeneralizedVarNode::Indirect(i) => self.read_varnode_indirect(&i),
        }
    }

    pub fn write<T: Into<GeneralizedVarNode>>(
        self,
        dest: T,
        val: BV<'ctx>,
    ) -> Result<Self, JingleError> {
        let gen_varnode: GeneralizedVarNode = dest.into();
        match gen_varnode {
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
        if self
            .jingle
            .get_space_info(dest.pointer_space_index)
            .unwrap()
            ._type
            == SpaceType::IPTR_CONSTANT
        {
            return Err(ConstantWrite);
        }
        let ptr = self.read_varnode(&dest.pointer_location)?;
        self.spaces[dest.pointer_space_index].write(&val, &ptr)?;
        Ok(self)
    }

    #[expect(unused)]
    fn write_varnode_metadata_indirect(
        &mut self,
        dest: &IndirectVarNode,
        val: BV<'ctx>,
    ) -> Result<(), JingleError> {
        if self
            .jingle
            .get_space_info(dest.pointer_space_index)
            .unwrap()
            ._type
            == SpaceType::IPTR_CONSTANT
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

    pub fn _eq(&self, other: &MemoryState<'ctx>) -> Bool<'ctx> {
        let mut terms = vec![];
        for (ours, theirs) in self.spaces.iter().zip(&other.spaces).skip(1) {
            if !ours._meta_eq(theirs) {
                return Bool::from_bool(self.jingle.z3, false);
            }
            terms.push(ours._eq(theirs))
        }
        let eq_terms: Vec<&Bool> = terms.iter().collect();
        Bool::and(self.jingle.z3, eq_terms.as_slice())
    }

    /// A helper function for Branch and CBranch.
    ///
    /// These two opcodes are able to perform p-code-relative branching, in which a
    /// CONSTANT branch target varnode is used to indicate a jump within p-code in the same
    /// machine instruction. In these cases, we DO want to enforce state constraints on the `unique`
    /// space.
    ///
    /// This function accepts the destination varnode of a jump and will conditionally reset the
    /// `unique` space iff the jump is NOT p-code-relative.
    fn conditional_clear_internal_space(&mut self, vn: &VarNode) {
        // todo! this is incorrect; the clearing needs to be tied to just the path where
        // the branch is taken
        if let Some(a) = self.jingle.get_space_info(vn.space_index) {
            // if this is a branch outside the machine instruction
            if a._type != SpaceType::IPTR_CONSTANT {
                // then reset the internal space
                self.clear_internal_space()
            }
        }
    }

    /// Sets the 'internal' space to be a new [BMCModeledSpace].
    ///
    /// The `unique` space contains 'scratch' data used to express the functionality of a machine
    /// instruction. It is purely a construction of the p-code encoding and is assumed to be unique
    /// to each machine instruction.
    ///
    /// Therefore, if modeling a jump to another instruction, it is necessary to replace this space
    /// with a fresh space, to prevent constraining its contents across machine instruction boundaries.
    pub fn clear_internal_space(&mut self) {
        for x in self.jingle.get_all_space_info() {
            let idx = x.index;
            if x._type == SpaceType::IPTR_INTERNAL {
                self.spaces[idx] = BMCModeledSpace::new(self.jingle.z3, x);
            }
        }
    }
}
