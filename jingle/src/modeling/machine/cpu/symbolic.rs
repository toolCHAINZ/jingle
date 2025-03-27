use crate::modeling::machine::cpu::concrete::{
    ConcretePcodeAddress, PcodeMachineAddress, PcodeOffset,
};
use crate::JingleError;
use jingle_sleigh::VarNode;
use z3::ast::{Ast, Bool, BV};
use z3::Context;

pub type SymbolicPcodeMachineAddress<'ctx> = BV<'ctx>;
pub type SymbolicPcodeOffset<'ctx> = BV<'ctx>;

// todo: add PcodeAddressSpace to Concrete and Symbolic addresses?
// probably necessary for harvard architecture modeling.
// ALSO: could be useful for callother.
#[derive(Debug)]
pub struct SymbolicPcodeAddress<'ctx> {
    pub(crate) machine: BV<'ctx>,
    pub(crate) pcode: BV<'ctx>,
}

impl<'ctx> SymbolicPcodeAddress<'ctx> {
    const MACHINE_SIZE_BITS: u32 = size_of::<PcodeMachineAddress>() as u32 * 8;
    const PCODE_SIZE_BITS: u32 = size_of::<PcodeOffset>() as u32 * 8;

    pub fn fresh(z3: &'ctx Context) -> Self {
        Self {
            machine: BV::fresh_const(z3, "pc", Self::MACHINE_SIZE_BITS),
            pcode: BV::fresh_const(z3, "ppc", Self::PCODE_SIZE_BITS),
        }
    }

    pub fn try_from_symbolic_dest(z3: &'ctx Context, bv: &BV<'ctx>) -> Result<Self, JingleError> {
        if bv.get_size() != Self::MACHINE_SIZE_BITS {
            Err(JingleError::InvalidBranchTargetSize)
        } else {
            Ok(SymbolicPcodeAddress {
                machine: bv.clone(),
                pcode: BV::from_u64(z3, 0u64, size_of::<PcodeOffset>() as u32 * 8),
            })
        }
    }

    fn extract_pcode(&self) -> &BV<'ctx> {
        &self.pcode
    }

    fn extract_machine(&self) -> &BV<'ctx> {
        &self.machine
    }
    pub fn concretize(&self) -> Option<ConcretePcodeAddress> {
        let pcode_offset = self.extract_pcode().simplify();
        let machine_addr = self.extract_machine().simplify();
        pcode_offset
            .as_u64()
            .zip(machine_addr.as_u64())
            .map(|(p, m)| ConcretePcodeAddress {
                machine: m,
                pcode: p as PcodeOffset,
            })
    }
    pub fn interpret_branch_dest_varnode(&self, vn: &VarNode) -> Self {
        match vn.is_const() {
            true => self.add_pcode_offset(vn.offset),
            false => ConcretePcodeAddress::from(vn.offset).symbolize(self.machine.get_ctx()),
        }
    }
    pub fn increment_pcode(&self) -> SymbolicPcodeAddress<'ctx> {
        self.add_pcode_offset(1)
    }
    fn add_pcode_offset(&self, offset: u64) -> SymbolicPcodeAddress<'ctx> {
        let pcode = self.extract_pcode() + offset;
        let machine = self.extract_machine().clone();
        SymbolicPcodeAddress { machine, pcode }
    }

    pub fn _eq(&self, other: &Self) -> Bool<'ctx> {
        self.machine._eq(&other.machine) & self.pcode._eq(&other.pcode)
    }
}
