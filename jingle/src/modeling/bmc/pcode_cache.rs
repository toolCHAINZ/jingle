use crate::modeling::bmc::machine::cpu::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::collections::{HashMap, HashSet};

pub struct PcodeCache {
    operations: HashMap<ConcretePcodeAddress, PcodeOperation>,
    pending_destinations: HashSet<ConcretePcodeAddress>,
    follow_calls: bool,
}
