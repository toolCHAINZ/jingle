use std::borrow::Borrow;
use crate::analysis::cfg::PcodeCfg;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome};

pub enum UnwoundLocation {
    UnwindError(ConcretePcodeAddress),
    Location(usize, ConcretePcodeAddress),
}


pub type UnwoundCfg = PcodeCfg<UnwoundLocation, PcodeOperation>;

impl AbstractState for UnwoundLocation {
    type SuccessorIter = ();

    fn merge(&mut self, other: &Self) -> MergeOutcome {
        todo!()
    }

    fn stop<'a, T: Iterator<Item=&'a Self>>(&'a self, states: T) -> bool {
        todo!()
    }

    fn transfer<B: Borrow<PcodeOperation>>(&self, opcode: B) -> Self::SuccessorIter {
        todo!()
    }
}