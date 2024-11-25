use std::ops::{Add, Deref};
use z3::ast::{Ast, BV};
use z3::Context;
use crate::JingleError;
use crate::modeling::bmc::machine::cpu::concrete::{ConcretePcodeAddress, PcodeMachineAddress, PcodeOffset};

// todo: add PcodeAddressSpace to Concrete and Symbolic addresses?
// probably necessary for harvard architecture modeling.
// ALSO: could be useful for callother.
#[derive(Debug, Eq, PartialEq)]
pub struct SymbolicPcodeAddress<'ctx>(pub(crate) BV<'ctx>);

impl<'ctx> Deref for SymbolicPcodeAddress<'ctx> {
    type Target = BV<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'ctx> SymbolicPcodeAddress<'ctx> {
    const MACHINE_TOP: u32 = size_of::<PcodeMachineAddress>() as u32 * 8;
    const PIVOT: u32 = size_of::<PcodeOffset>() as u32 * 8;

    pub fn fresh(z3: &'ctx Context) -> Self {
        Self(BV::fresh_const(z3, "pc", Self::MACHINE_TOP))
    }

    pub fn try_from_symbolic_dest(z3: &'ctx Context, bv: &BV<'ctx>) -> Result<Self, JingleError> {
        if bv.get_size() != Self::MACHINE_TOP {
            Err(JingleError::InvalidBranchTargetSize)
        } else {
            Ok(SymbolicPcodeAddress(bv.concat(&BV::from_u64(
                z3,
                0u64,
                size_of::<PcodeOffset>() as u32 * 8,
            ))))
        }
    }

    fn extract_pcode(&self) -> BV<'ctx> {
        self.extract(Self::PIVOT - 1, 0)
    }

    fn extract_machine(&self) -> BV<'ctx> {
        self.extract(Self::MACHINE_TOP + Self::PIVOT - 1, Self::PIVOT)
    }
    pub fn concretize(&self) -> Option<ConcretePcodeAddress> {
        let pcode_offset = self.extract_pcode().simplify();
        let machine_addr = self.extract_machine().simplify();
        pcode_offset
            .as_u64()
            .zip(machine_addr.as_u64())
            .map(|(p, m)| ConcretePcodeAddress {
                machine: m,
                pcode: p as PcodeOffset
            })
    }

    pub fn increment_pcode(&self) -> SymbolicPcodeAddress<'ctx> {
        let ext = self.extract_pcode().add(1u64);
        let machine = self.extract_machine();
        SymbolicPcodeAddress(machine.concat(&ext))
    }
}