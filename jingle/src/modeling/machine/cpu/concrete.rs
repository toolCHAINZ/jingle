use crate::analysis::cpa::state::Successor;
use crate::modeling::machine::cpu::symbolic::SymbolicPcodeAddress;
use jingle_sleigh::{PcodeOperation, VarNode};
use std::fmt::{Display, Formatter, LowerHex};
use std::iter::{empty, once};
use z3::ast::BV;

pub type PcodeMachineAddress = u64;
pub type PcodeOffset = u8;
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ConcretePcodeAddress {
    pub(crate) machine: PcodeMachineAddress,
    pub(crate) pcode: PcodeOffset,
}

impl Display for ConcretePcodeAddress {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.machine, self.pcode)
    }
}

impl LowerHex for ConcretePcodeAddress {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:x}:{:x}", self.machine, self.pcode)
    }
}

impl ConcretePcodeAddress {
    pub fn new(machine: PcodeMachineAddress, pcode: PcodeOffset) -> Self {
        Self { machine, pcode }
    }

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
    pub fn symbolize(&self) -> SymbolicPcodeAddress {
        SymbolicPcodeAddress {
            machine: BV::from_u64(self.machine, size_of::<PcodeMachineAddress>() as u32 * 8),
            pcode: BV::from_u64(self.pcode as u64, size_of::<PcodeOffset>() as u32 * 8),
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

impl From<&PcodeMachineAddress> for ConcretePcodeAddress {
    fn from(value: &PcodeMachineAddress) -> Self {
        Self {
            machine: *value,
            pcode: 0,
        }
    }
}
/// Simple in-function transition handling, transitioning from one address to the
/// next per uninterpreted/unanalyzed pcode. The implementation is used for
/// the initial exploration of a CFG. The analyses built around this have
/// support for more fine-tuned behavior.
///
/// This one assumes:
/// * Calls return
/// * Both sides of a conditional can be taken
/// * All Indirect branches transition to Top
impl ConcretePcodeAddress {
    pub fn transfer<'a>(&'a self, op: &PcodeOperation) -> Successor<'a, Self> {
        match op {
            PcodeOperation::Branch { input } => {
                once(ConcretePcodeAddress::from(input.offset)).into()
            }
            PcodeOperation::CBranch { input0, .. } => {
                let dest = ConcretePcodeAddress::resolve_from_varnode(input0, *self);
                let fallthrough = self.next_pcode();
                once(dest).chain(once(fallthrough)).into()
            }
            PcodeOperation::Call { .. } | PcodeOperation::CallOther { .. } => {
                once(self.next_pcode()).into()
            }
            PcodeOperation::Return { .. }
            | PcodeOperation::CallInd { .. }
            | PcodeOperation::BranchInd { .. } => empty().into(),
            _ => once(self.next_pcode()).into(),
        }
    }
}
