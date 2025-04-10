use crate::modeling::machine::cpu::symbolic::SymbolicPcodeAddress;
use jingle_sleigh::VarNode;
use std::fmt::{Display, Formatter, LowerHex};
use z3::Context;
use z3::ast::BV;

pub type PcodeMachineAddress = u64;
pub type PcodeOffset = u8;
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ConcretePcodeAddress {
    pub(crate) machine: PcodeMachineAddress,
    pub(crate) pcode: PcodeOffset,
}

impl Display for ConcretePcodeAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.machine, self.pcode)
    }
}

impl LowerHex for ConcretePcodeAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}:{:x}", self.machine, self.pcode)
    }
}

impl ConcretePcodeAddress {
    pub fn next_pcode(&self) -> Self {
        self.add_pcode_offset(1)
    }

    pub fn machine(&self) -> PcodeMachineAddress {
        self.machine
    }

    pub fn pcode(&self) -> PcodeOffset {
        self.pcode
    }

    pub(crate) fn add_pcode_offset(&self, off: PcodeOffset) -> Self {
        Self {
            machine: self.machine,
            pcode: self.pcode.wrapping_add(off),
        }
    }
    pub fn symbolize<'ctx>(&self, z3: &'ctx Context) -> SymbolicPcodeAddress<'ctx> {
        SymbolicPcodeAddress {
            machine: BV::from_u64(
                z3,
                self.machine,
                size_of::<PcodeMachineAddress>() as u32 * 8,
            ),
            pcode: BV::from_u64(z3, self.pcode as u64, size_of::<PcodeOffset>() as u32 * 8),
        }
    }

    pub fn resolve_from_varnode(vn: &VarNode, loc: ConcretePcodeAddress) -> Self {
        if vn.is_const() {
            // relative jump
            loc.add_pcode_offset(vn.offset as u8)
        } else {
            // absolute jump
            ConcretePcodeAddress {
                machine: vn.offset,
                pcode: 0,
            }
        }
    }
}

impl From<PcodeMachineAddress> for ConcretePcodeAddress {
    fn from(value: PcodeMachineAddress) -> Self {
        Self {
            machine: value,
            pcode: 0,
        }
    }
}
