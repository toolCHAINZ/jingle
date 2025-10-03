use crate::analysis::cfg::PcodeCfg;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;

pub enum UnwoundLocation {
    UnwindError(ConcretePcodeAddress),
    Location(usize, ConcretePcodeAddress),
}


pub type UnwoundCfg = PcodeCfg<UnwoundLocation, PcodeOperation>;
