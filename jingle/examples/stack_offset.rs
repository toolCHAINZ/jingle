#![allow(unused)]

use jingle::analysis::Analysis;
use jingle::analysis::back_edge::{BackEdgeAnalysis, BackEdges};
use jingle::analysis::cfg::PcodeCfg;
use jingle::analysis::compound::CompoundState;
use jingle::analysis::cpa::lattice::simple::SimpleLattice;
use jingle::analysis::cpa::state::AbstractState;
use jingle::analysis::cpa::{ConfigurableProgramAnalysis, RunnableConfigurableProgramAnalysis};
use jingle::analysis::pcode_store::PcodeStore;
use jingle::analysis::stack_offset::StackOffsetState;
use jingle::analysis::unwinding::{UnwindingCpaState, UnwoundLocation};
use jingle_sleigh::context::image::gimli::load_with_gimli;
use jingle_sleigh::PcodeOperation;
use std::env;

const FUNC_LINE: u64 = 0x100000460;
const FUNC_BRANCH: u64 = 0x100000480;
const FUNC_SWITCH: u64 = 0x1000004a0;
const FUNC_LOOP: u64 = 0x100000548;
const FUNC_NESTED: u64 = 0x100000588;
const FUNC_GOTO: u64 = 0x100000610;



impl Analysis for (Unwou) {
    type Output = PcodeCfg<UnwoundLocation, (PcodeOperation, i64)>;
    type Input = jingle::modeling::machine::cpu::concrete::ConcretePcodeAddress;

    fn run<T: PcodeStore, I: Into<Self::Input>>(
        &mut self,
        store: T,
        initial_state: I,
    ) -> Self::Output {
        use jingle::modeling::machine::cpu::concrete::ConcretePcodeAddress;

        let addr: ConcretePcodeAddress = initial_state.into();
        let info = store.info();

        // Get back edges for unwinding
        let bes = BackEdgeAnalysis.make_initial_state(addr);
        let back_edges = BackEdgeAnalysis.run(&store, bes);

        // Get stack pointer varnode for this architecture
        // Common stack pointer register names by architecture
        let stack_pointer = info
            .register("RSP")  // x86-64
            .or_else(|| info.register("ESP"))  // x86-32
            .or_else(|| info.register("SP"))   // ARM, MIPS, etc.
            .or_else(|| info.register("r13"))  // ARM alternative
            .cloned()
            .expect("Could not find stack pointer register");

        // Create initial compound state
        let unwinding_state =
            UnwindingCpaState::new(addr, back_edges, self.unwinding_bound, self.max_step_bound);
        let stack_state = StackOffsetState::new(stack_pointer);

        let initial = CompoundState::new(
            SimpleLattice::Value(unwinding_state),
            stack_state,
        );

        // Run the compound CPA
        let mut cpa = CompoundStackOffsetCPA::new(info.clone());
        let _ = cpa.run_cpa(&initial, &store);

        cpa.cfg
    }

    fn make_initial_state(&self, addr: Self::Input) -> Self::Input {
        addr
    }
}

fn main() {
    let bin_path = env::home_dir()
        .unwrap()
        .join("Documents/test_funcs/build/example");
    let loaded = load_with_gimli(bin_path, "/Applications/ghidra").unwrap();

    // Run the compound analysis
    let mut analysis = CompoundStackOffsetAnalysis::new(10);
    let result = analysis.run(&loaded, analysis.make_initial_state(FUNC_NESTED.into()));

    // Print results
    println!("Stack Offset Analysis Results:");
    println!("==============================\n");

    let nodes: Vec<_> = result.nodes().collect();
    for node in &nodes {
        println!("Location: {:x}", node.location());
    }

    println!("\nEdges with stack offsets:");
    for edge in result.graph().edge_references() {
        let source = result.graph().node_weight(edge.source()).unwrap();
        let target = result.graph().node_weight(edge.target()).unwrap();
        let (op, offset) = edge.weight();
        println!(
            "{:x} -> {:x}: {} (stack offset: {})",
            source.location(),
            target.location(),
            op,
            offset
        );
    }

    let leaf_nodes: Vec<_> = result.leaf_nodes().collect();
    println!("\nLeaf nodes: {} total", leaf_nodes.len());
    for leaf in &leaf_nodes {
        println!("  {:x}", leaf.location());
    }
}

