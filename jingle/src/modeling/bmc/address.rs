use std::ops::Deref;
use z3::ast::{Ast, BV};
use z3::Context;

pub type PcodeMachineAddress = u64;
pub type PcodeOffset = u8;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ConcretePcodeAddress(PcodeMachineAddress, PcodeOffset);

pub struct SymbolicPcodeAddress<'ctx>(BV<'ctx>);

impl<'ctx> Deref for SymbolicPcodeAddress<'ctx> {
    type Target = BV<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ConcretePcodeAddress {
    pub fn symbolize<'ctx>(&self, z3: &'ctx Context) -> SymbolicPcodeAddress<'ctx> {
        SymbolicPcodeAddress(BV::concat(
            &BV::from_u64(z3, self.0, size_of::<PcodeMachineAddress>() as u32 * 8),
            &BV::from_u64(z3, self.1 as u64, size_of::<PcodeOffset>() as u32 * 8),
        ))
    }
}

impl From<u64> for ConcretePcodeAddress {
    fn from(value: u64) -> Self {
        Self(value, 0)
    }
}

impl<'ctx> SymbolicPcodeAddress<'ctx> {
    const MACHINE_TOP: u32 = size_of::<PcodeMachineAddress>() as u32 * 8;
    const PIVOT: u32 = size_of::<PcodeOffset>() as u32 * 8;
    pub fn concretize(&self) -> Option<ConcretePcodeAddress> {
        let pcode_offset = self.extract(Self::PIVOT - 1, 0).simplify();
        let machine_addr = self
            .extract(Self::MACHINE_TOP + Self::PIVOT - 1, Self::PIVOT)
            .simplify();

        if pcode_offset.is_const() && machine_addr.is_const() {
            let pcode_offset = pcode_offset.as_u64().unwrap() as PcodeOffset;
            let machine_addr = machine_addr.as_u64().unwrap();
            Some(ConcretePcodeAddress(machine_addr, pcode_offset))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::modeling::bmc::address::ConcretePcodeAddress;
    use z3::{Config, Context};

    #[test]
    fn address_round_trip() {
        let addr = ConcretePcodeAddress(0xdeadbeefcafebabe, 0x50);
        let z3 = Context::new(&Config::new());
        let symbolized = addr.symbolize(&z3);
        let new_concrete = symbolized.concretize().unwrap();
        assert_eq!(addr, new_concrete)
    }
}
