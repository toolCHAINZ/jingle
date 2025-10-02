use crate::analysis::cfg::PcodeCfg;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;

pub struct UnwoundLocation {
    count: usize,
    location: ConcretePcodeAddress,
}

pub type UnwoundCfg = PcodeCfg<UnwoundLocation, PcodeOperation>;
