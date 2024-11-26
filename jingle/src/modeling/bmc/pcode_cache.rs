use crate::modeling::bmc::context::BMCJingleContext;
use crate::modeling::bmc::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::branch::PcodeBranchDestination;
use jingle_sleigh::context::loaded::LoadedSleighContext;
use jingle_sleigh::{Instruction, PcodeOperation};
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Default, Clone)]
pub struct SimpleOperationCache {
    /// Addresses and operations that have already been visited
    operations: BTreeMap<ConcretePcodeAddress, PcodeOperation>,
    /// Addresses that we need to visit
    pending_destinations: BTreeSet<ConcretePcodeAddress>,
    /// Addresses that we have already determined contain indirect jumps that we must model
    indirect_frontier: HashSet<ConcretePcodeAddress>,
    /// Addresses that are branched to but do not generate any pcode. Will be needed
    /// for assertions
    illegal_addresses: HashSet<ConcretePcodeAddress>,
}

impl SimpleOperationCache {
    fn add_operation(&mut self, addr: ConcretePcodeAddress, op: &PcodeOperation) {
        // if we have already visited this address, this should be a noop
        if let std::collections::btree_map::Entry::Vacant(e) = self.operations.entry(addr) {
            // mark that we have visited the operation and remove it from "next"
            e.insert(op.clone());
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

    fn process_instruction(&mut self, instr: &Instruction) {
        let mut addr = ConcretePcodeAddress::from(instr.address);
        for op in &instr.ops {
            self.add_operation(addr, op);
            addr = addr.next_pcode();
        }
    }

    pub fn initialize(start_address: ConcretePcodeAddress, ctx: LoadedSleighContext<'_>) -> Self {
        let mut s = Self::default();
        s.pending_destinations.insert(start_address);
        while !s.pending_destinations.is_empty() {
            let addr = s.pending_destinations.pop_first().unwrap();
            if addr.pcode() != 0 {
                panic!("This should never happen")
            }
            if let Some(a) = ctx.instruction_at(addr.machine()) {
                s.process_instruction(&a);
            } else {
                s.illegal_addresses.insert(start_address);
            }
        }
        s
    }

    pub fn merge(&mut self, other: Self) {
        self.illegal_addresses.extend(&other.illegal_addresses);
        self.indirect_frontier.extend(&other.indirect_frontier);
        self.pending_destinations
            .extend(&other.pending_destinations);
        self.operations.extend(other.operations);
    }
}
