mod relations;
pub mod space;

use crate::JingleError::{
    ConstantWrite, IndirectConstantRead, MismatchedWordSize, UnexpectedArraySort, UnmodeledSpace,
    ZeroSizedVarnode,
};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use crate::modeling::machine::memory::space::BMCModeledSpace;
use crate::varnode::ResolvedVarnode;
use crate::{JingleContext, JingleError};
use jingle_sleigh::{
    ArchInfoProvider, GeneralizedVarNode, IndirectVarNode, SpaceInfo, SpaceType, VarNode,
};
use std::borrow::Borrow;
use std::ops::Add;
use z3::ast::{Array, BV, Bool};

/// Represents the modeled combined memory state of the system. State
/// is represented with Z3 formulas built up as select and store operations
/// on an initial state
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryState {
    jingle: JingleContext,
    spaces: Vec<BMCModeledSpace>,
}

impl MemoryState {
    pub fn fresh(jingle: &JingleContext) -> Self {
        let jingle = jingle.clone();
        let spaces: Vec<BMCModeledSpace> = jingle
            .get_all_space_info()
            .map(BMCModeledSpace::new)
            .collect();
        Self { jingle, spaces }
    }

    pub fn fresh_for_address<T: Borrow<ConcretePcodeAddress>>(
        jingle: &JingleContext,
        addr: T,
    ) -> Self {
        let addr = addr.borrow();
        let jingle = jingle.clone();
        let spaces: Vec<BMCModeledSpace> = jingle
            .get_all_space_info()
            .map(|s| BMCModeledSpace::new_for_address(s, addr))
            .collect();
        Self { jingle, spaces }
    }

    pub fn get_space(&self, idx: usize) -> Result<&Array, JingleError> {
        self.spaces
            .get(idx)
            .map(|u| u.get_space())
            .ok_or(UnmodeledSpace)
    }

    fn read_varnode(&self, varnode: &VarNode) -> Result<BV, JingleError> {
        let space = self
            .jingle
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
                arr.read(&offset, varnode.size)
            }
        }
    }

    fn read_varnode_indirect(&self, indirect: &IndirectVarNode) -> Result<BV, JingleError> {
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
    ) -> Result<BV, JingleError> {
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

    pub fn read<T: Into<GeneralizedVarNode>>(&self, vn: T) -> Result<BV, JingleError> {
        let gen_varnode: GeneralizedVarNode = vn.into();
        match gen_varnode {
            GeneralizedVarNode::Direct(d) => self.read_varnode(&d),
            GeneralizedVarNode::Indirect(i) => self.read_varnode_indirect(&i),
        }
    }

    pub fn write<T: Into<GeneralizedVarNode>>(self, dest: T, val: BV) -> Result<Self, JingleError> {
        let gen_varnode: GeneralizedVarNode = dest.into();
        match gen_varnode {
            GeneralizedVarNode::Direct(d) => self.write_varnode(&d, val),
            GeneralizedVarNode::Indirect(i) => self.write_varnode_indirect(&i, val),
        }
    }

    /// Model a write to a [VarNode] on top of the current context.
    fn write_varnode(mut self, dest: &VarNode, val: BV) -> Result<Self, JingleError> {
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
                space.write(&val, &BV::from_u64(dest.offset, info.index_size_bytes * 8))?;
                Ok(self)
            }
        }
    }

    /// Model a write to an [IndirectVarNode] on top of the current context.
    fn write_varnode_indirect(
        mut self,
        dest: &IndirectVarNode,
        val: BV,
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
        val: BV,
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

    pub fn read_resolved(&self, vn: &ResolvedVarnode) -> Result<BV, JingleError> {
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

    pub fn _eq(&self, other: &MemoryState, machine_eq: &Bool) -> Bool {
        let mut terms = vec![];
        // skipping one space because the CONST space is ALWAYS first and we don't need
        // to encode equality of CONST
        for (ours, theirs) in self.spaces.iter().zip(&other.spaces).skip(1) {
            if !ours._meta_eq(theirs) {
                return Bool::from_bool(false);
            }
            // If we're dealing with an internal space
            if ours.get_type() == SpaceType::IPTR_INTERNAL {
                // then if both spaces have the same symbolic machine address, they are equal
                // this expresses the "resetting" of the internal space between different
                // machine instructions
                terms.push(machine_eq.implies(ours._eq(theirs)))
            } else {
                // otherwise, we simply assert that the spaces are equal
                terms.push(ours._eq(theirs))
            }
        }
        let eq_terms: Vec<&Bool> = terms.iter().collect();
        Bool::and(eq_terms.as_slice())
    }
}
