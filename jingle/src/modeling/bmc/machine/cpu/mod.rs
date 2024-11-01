mod relations;

use crate::JingleError;
use jingle_sleigh::{SpaceManager, VarNode};
use std::ops::{Add, Deref};
use z3::ast::{Ast, BV};
use z3::Context;

pub type PcodeMachineAddress = u64;
pub type PcodeOffset = u8;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ConcretePcodeAddress(PcodeMachineAddress, PcodeOffset);

#[derive(Debug, Eq, PartialEq)]
pub struct SymbolicPcodeAddress<'ctx>(BV<'ctx>);

impl<'ctx> Deref for SymbolicPcodeAddress<'ctx> {
    type Target = BV<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ConcretePcodeAddress {
    fn add_pcode_offset(&self, off: PcodeOffset) -> Self {
        Self(self.0, self.1.wrapping_add(off))
    }
    pub fn symbolize<'ctx>(&self, z3: &'ctx Context) -> SymbolicPcodeAddress<'ctx> {
        SymbolicPcodeAddress(BV::concat(
            &BV::from_u64(z3, self.0, size_of::<PcodeMachineAddress>() as u32 * 8),
            &BV::from_u64(z3, self.1 as u64, size_of::<PcodeOffset>() as u32 * 8),
        ))
    }

    pub fn resolve_from_varnode<T: SpaceManager>(
        vn: &VarNode,
        sp: &T,
        loc: ConcretePcodeAddress,
    ) -> Self {
        if vn.space_index == sp.get_code_space_idx() {
            // relative jump
            loc.add_pcode_offset(vn.offset as u8)
        } else {
            // absolute jump
            ConcretePcodeAddress(vn.offset, 0)
        }
    }
}

impl From<PcodeMachineAddress> for ConcretePcodeAddress {
    fn from(value: PcodeMachineAddress) -> Self {
        Self(value, 0)
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
            .map(|(p, m)| ConcretePcodeAddress(m, p as PcodeOffset))
    }

    pub fn increment_pcode(&self) -> SymbolicPcodeAddress<'ctx> {
        let ext = self.extract_pcode().add(1u64);
        let machine = self.extract_machine();
        SymbolicPcodeAddress(machine.concat(&ext))
    }
}

#[cfg(test)]
mod tests {
    use crate::modeling::bmc::machine::cpu::{
        ConcretePcodeAddress, PcodeOffset, SymbolicPcodeAddress,
    };
    use z3::ast::BV;
    use z3::{Config, Context};

    #[test]
    fn address_round_trip() {
        let addr = ConcretePcodeAddress(0xdeadbeefcafebabe, 0x50);
        let z3 = Context::new(&Config::new());
        let symbolized = addr.symbolize(&z3);
        let new_concrete = symbolized.concretize().unwrap();
        assert_eq!(addr, new_concrete)
    }

    #[test]
    fn increment_pcode_addr() {
        let addr = ConcretePcodeAddress(0, 0);
        let z3 = Context::new(&Config::new());
        let symbolized = addr.symbolize(&z3);
        assert_eq!(symbolized.concretize().unwrap(), ConcretePcodeAddress(0, 0));
        let plus_1 = symbolized.increment_pcode();
        assert_eq!(plus_1.concretize().unwrap(), ConcretePcodeAddress(0, 1));
        let symbolized = ConcretePcodeAddress(0, 0xff).symbolize(&z3);
        let plus_1 = symbolized.increment_pcode();
        assert_eq!(plus_1.concretize().unwrap(), ConcretePcodeAddress(0, 0));
    }

    #[test]
    fn create_symbolic_addr() {
        let z3 = Context::new(&Config::new());
        let addr = BV::from_u64(&z3, 0xdeadbeef, 64);
        let wrong = BV::from_u64(&z3, 0xdeadbeef, 65);

        let sym = SymbolicPcodeAddress::try_from_symbolic_dest(&z3, &addr).unwrap();
        assert_eq!(
            sym.concretize().unwrap(),
            ConcretePcodeAddress(0xdeadbeef, 0)
        );

        let sym = SymbolicPcodeAddress::try_from_symbolic_dest(&z3, &wrong);
        assert!(matches!(sym, Err(_)));
    }

    #[test]
    fn test_relative_math() {
        let addr = ConcretePcodeAddress(4, 4);
        let dec1 = addr.add_pcode_offset(-1i8 as PcodeOffset);
        let add1 = addr.add_pcode_offset(1i8 as PcodeOffset);
        let add255 = addr.add_pcode_offset(255);

        assert_eq!(dec1, ConcretePcodeAddress(4, 3));
        assert_eq!(add1, ConcretePcodeAddress(4, 5));
        assert_eq!(dec1, add255);
    }
}
