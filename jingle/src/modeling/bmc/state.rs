use std::ops::Add;
use z3::ast::{Array, Ast, Bool, BV};
use jingle_sleigh::{GeneralizedVarNode, IndirectVarNode, SpaceInfo, SpaceType, SpaceManager, VarNode};
use crate::JingleError;
use crate::JingleError::{ConstantWrite, IndirectConstantRead, MismatchedWordSize, UnexpectedArraySort, UnmodeledSpace, ZeroSizedVarnode};
use crate::modeling::bmc::context::JingleContext;
use crate::modeling::bmc::space::BMCModeledSpace;
use crate::varnode::ResolvedVarnode;

/// Represents the modeled combined memory state of the system. State
/// is represented with Z3 formulas built up as select and store operations
/// on an initial state
#[derive(Clone, Debug)]
pub struct MemoryState<'a, 'ctx, 'sl> {
    jingle: &'a JingleContext<'ctx, 'sl>,
    spaces: Vec<BMCModeledSpace<'ctx>>,
}

impl<'a, 'ctx, 'sl> SpaceManager for MemoryState<'a, 'ctx, 'sl> {
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

impl<'a, 'ctx, 'sl> MemoryState<'a, 'ctx, 'sl> {
    pub fn new(jingle: &'a JingleContext<'ctx, 'sl>) -> Self {
        let spaces: Vec<BMCModeledSpace<'ctx>> = jingle
            .get_all_space_info()
            .iter()
            .map(|s| BMCModeledSpace::new(&jingle.z3, s))
            .collect();
        Self { jingle, spaces }
    }

    pub fn get_space(&self, idx: usize) -> Result<&Array<'ctx>, JingleError> {
        self.spaces
            .get(idx)
            .map(|u| u.get_space())
            .ok_or(UnmodeledSpace)
    }

    pub fn read_varnode(&self, varnode: &VarNode) -> Result<BV<'a>, JingleError> {
        let space = self
            .get_space_info(varnode.space_index)
            .ok_or(UnmodeledSpace)?;
        match space._type {
            SpaceType::IPTR_CONSTANT => Ok(BV::from_i64(
                &self.jingle.z3,
                varnode.offset as i64,
                (varnode.size * 8) as u32,
            )),
            _ => {
                let offset = BV::from_i64(
                    &self.jingle.z3,
                    varnode.offset as i64,
                    space.index_size_bytes * 8,
                );
                let arr = self.spaces.get(varnode.space_index).ok_or(UnmodeledSpace)?;
                arr.read(&offset, varnode.size)
            }
        }
    }
    

    pub fn read_varnode_indirect(
        &self,
        indirect: &IndirectVarNode,
    ) -> Result<BV<'a>, JingleError> {
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

    pub fn read_varnode_metadata_indirect(
        &self,
        indirect: &IndirectVarNode,
    ) -> Result<BV<'a>, JingleError> {
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

    pub fn read(&self, vn: GeneralizedVarNode) -> Result<BV<'a>, JingleError> {
        match vn {
            GeneralizedVarNode::Direct(d) => self.read_varnode(&d),
            GeneralizedVarNode::Indirect(i) => self.read_varnode_indirect(&i),
        }
    }

    /// Model a write to a [VarNode] on top of the current context.
    pub fn write_varnode<'b>(
        &'b mut self,
        dest: &VarNode,
        val: BV<'b>,
    ) -> Result<(), JingleError> {
        if dest.size as u32 * 8 != val.get_size() {
            return Err(MismatchedWordSize);
        }
        match self.jingle.get_space_info(dest.space_index).unwrap()._type {
            SpaceType::IPTR_CONSTANT => Err(ConstantWrite),
            _ => {
                let space = self
                    .spaces
                    .get_mut(dest.space_index)
                    .ok_or(UnmodeledSpace)?;
                space.write(
                    &val,
                    &BV::from_u64(
                        &self.jingle.z3,
                        dest.offset,
                        self.get_space_info(dest.space_index).unwrap().index_size_bytes * 8,
                    ),
                )?;
                Ok(())
            }
        }
    }
    

    /// Model a write to an [IndirectVarNode] on top of the current context.
    pub fn write_varnode_indirect<'b>(
        &'b mut self,
        dest: &IndirectVarNode,
        val: BV<'b>,
    ) -> Result<(), JingleError> {
        if self.get_space_info(dest.pointer_space_index).unwrap()._type == SpaceType::IPTR_CONSTANT {
            return Err(ConstantWrite);
        }
        let ptr = self.read_varnode(&dest.pointer_location)?;
        self.spaces[dest.pointer_space_index].write(&val, &ptr)?;
        Ok(())
    }

    pub fn write_varnode_metadata_indirect<'b>(
        &'b mut self,
        dest: &IndirectVarNode,
        val: BV<'b>,
    ) -> Result<(), JingleError> {
        if self.get_space_info(dest.pointer_space_index).unwrap()._type == SpaceType::IPTR_CONSTANT {
            return Err(ConstantWrite);
        }
        let ptr = self.read_varnode(&dest.pointer_location)?;
        self.spaces[dest.pointer_space_index].write(&val, &ptr)?;
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
    

    pub fn _eq(&self, other: &MemoryState) -> Result<Bool<'ctx>, JingleError> {
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

