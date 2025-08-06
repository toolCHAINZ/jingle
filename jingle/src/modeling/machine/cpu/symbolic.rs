use crate::JingleError;
use crate::modeling::concretize::ConcretizationIterator;
use crate::modeling::machine::cpu::concrete::{
    ConcretePcodeAddress, PcodeMachineAddress, PcodeOffset,
};
use jingle_sleigh::VarNode;
use z3::Context;
use z3::ast::{Ast, BV, Bool};

pub type SymbolicPcodeMachineAddress = BV;
pub type SymbolicPcodeOffset = BV;

// todo: add PcodeAddressSpace to Concrete and Symbolic addresses?
// probably necessary for harvard architecture modeling.
// ALSO: could be useful for callother.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolicPcodeAddress {
    pub(crate) machine: BV,
    pub(crate) pcode: BV,
}

impl SymbolicPcodeAddress {
    const MACHINE_SIZE_BITS: u32 = size_of::<PcodeMachineAddress>() as u32 * 8;
    const PCODE_SIZE_BITS: u32 = size_of::<PcodeOffset>() as u32 * 8;

    pub fn fresh(z3: &Context) -> Self {
        Self {
            machine: BV::fresh_const(z3, "pc", Self::MACHINE_SIZE_BITS),
            pcode: BV::fresh_const(z3, "ppc", Self::PCODE_SIZE_BITS),
        }
    }

    pub fn try_from_symbolic_dest(z3: &Context, bv: &BV) -> Result<Self, JingleError> {
        if bv.get_size() != Self::MACHINE_SIZE_BITS {
            Err(JingleError::InvalidBranchTargetSize)
        } else {
            Ok(SymbolicPcodeAddress {
                machine: bv.clone(),
                pcode: BV::from_u64(z3, 0u64, size_of::<PcodeOffset>() as u32 * 8),
            })
        }
    }

    fn extract_pcode(&self) -> &BV {
        &self.pcode
    }

    fn extract_machine(&self) -> &BV {
        &self.machine
    }

    pub fn concretize_with_assertions<T: Iterator<Item = Bool>>(
        &self,
        s: T,
    ) -> ConcretizationIterator<Self> {
        ConcretizationIterator::new_with_assertions(s, self)
    }

    pub fn interpret_branch_dest_varnode(&self, vn: &VarNode) -> Self {
        match vn.is_const() {
            true => self.add_pcode_offset(vn.offset),
            false => ConcretePcodeAddress::from(vn.offset).symbolize(self.machine.get_ctx()),
        }
    }
    pub fn increment_pcode(&self) -> SymbolicPcodeAddress {
        self.add_pcode_offset(1)
    }
    fn add_pcode_offset(&self, offset: u64) -> SymbolicPcodeAddress {
        let pcode = self.extract_pcode() + offset;
        let machine = self.extract_machine().clone();
        SymbolicPcodeAddress { machine, pcode }
    }

    pub fn _eq(&self, other: &Self) -> Bool {
        self.machine._eq(&other.machine) & self.pcode._eq(&other.pcode)
    }

    pub fn simplify(&self) -> Self {
        let machine = self.machine.simplify();
        let pcode = self.pcode.simplify();
        SymbolicPcodeAddress { machine, pcode }
    }

    pub fn as_concrete(&self) -> Option<ConcretePcodeAddress> {
        if let Some(machine) = self.machine.simplify().as_u64() {
            if let Some(pcode) = self.pcode.simplify().as_u64() {
                return Some(ConcretePcodeAddress {
                    machine,
                    pcode: pcode as PcodeOffset,
                });
            }
        }
        None
    }
}
