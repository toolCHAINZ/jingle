use crate::modeling::bmc::machine::cpu::ConcretePcodeAddress;
use jingle_sleigh::branch::PcodeBranchDestination;
use jingle_sleigh::PcodeOperation;
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct SimpleOperationCache {
    /// Addresses and operations that have already been visited
    operations: BTreeMap<ConcretePcodeAddress, PcodeOperation>,
    /// Addresses that we need to visit
    pending_destinations: HashSet<ConcretePcodeAddress>,
    /// Addresses that we have already determined contain indirect jumps that we must model
    indirect_frontier: HashSet<ConcretePcodeAddress>,
}

impl SimpleOperationCache {
    fn add_operation(&mut self, addr: ConcretePcodeAddress, op: &PcodeOperation) {
        // if we have already visited this address, this should be a noop
        if !self.operations.contains_key(&addr) {
            // mark that we have visited the operation and remove it from "next"
            self.operations.insert(addr, op.clone());
            self.pending_destinations.remove(&addr);
            // look for more things to add
            if let Some(dest) = op.branch_destination() {
                match dest {
                    PcodeBranchDestination::Branch(dest)
                    | PcodeBranchDestination::Conditional(dest)
                    | PcodeBranchDestination::Call(dest) => {
                        // there's a concrete destination, translate it to a pcode address
                        let dest = ConcretePcodeAddress::resolve_from_varnode(&dest, addr);
                        // check if we've visited it
                        if !self.operations.contains_key(&dest) {
                            // if not, add it to the pile
                            self.pending_destinations.insert(dest);
                        }
                    }
                    PcodeBranchDestination::IndirectBranch(_)
                    | PcodeBranchDestination::IndirectCall(_) => {
                        // indirect jump here, keep track of THIS address to come back to later
                        self.indirect_frontier.insert(addr);
                    }
                    // assume we terminate at returns
                    PcodeBranchDestination::Return(_) => {}
                }
            }
        }
    }
}
