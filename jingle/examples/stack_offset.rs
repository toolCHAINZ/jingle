#![allow(unused)]

use jingle::analysis::Analysis;
use jingle::analysis::cfg::PcodeCfg;
use jingle::analysis::compound::CompoundState;
use jingle::analysis::cpa::lattice::simple::SimpleLattice;
use jingle::analysis::cpa::{ConfigurableProgramAnalysis, RunnableConfigurableProgramAnalysis};
use jingle::analysis::pcode_store::PcodeStore;
use jingle::analysis::stack_offset::{StackOffsetState, StackOffsetLattice};
use jingle::analysis::unwinding::{UnwindingCpaState, UnwoundLocation};
use jingle::analysis::back_edge::{BackEdgeCPA, BackEdgeState, BackEdges};
use jingle::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use jingle_sleigh::context::image::gimli::load_with_gimli;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use jingle::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use std::collections::HashMap;
use std::env;

const FUNC_LINE: u64 = 0x100000460;
const FUNC_BRANCH: u64 = 0x100000480;
const FUNC_SWITCH: u64 = 0x1000004a0;
const FUNC_LOOP: u64 = 0x100000548;
const FUNC_NESTED: u64 = 0x100000588;
const FUNC_GOTO: u64 = 0x100000610;


fn main() {
    let bin_path = env::home_dir()
        .unwrap()
        .join("Documents/test_funcs/build/example");
    let loaded = load_with_gimli(bin_path, "/Applications/ghidra").unwrap();

    // Create and run the compound analysis
    let mut analysis = CompoundStackOffsetCPA::new(
        loaded.info(),
        10,  // unwinding bound
        Some(100),  // max steps
    );
    
    let (cfg, stack_offsets) = analysis.run(&loaded, analysis.make_initial_state(FUNC_NESTED.into()));

    // Print results
    println!("Stack Offset Analysis Results:");
    println!("==============================\n");

    // Collect and sort locations for consistent output
    let mut locations: Vec<_> = cfg.nodes().collect();
    locations.sort_by_key(|loc| *loc.location());

    println!("Locations and their stack offsets:");
    for loc in &locations {
        let offset_str = stack_offsets
            .get(loc)
            .map(|off| format!("{:+}", off))
            .unwrap_or_else(|| "unknown".to_string());
        println!("  {:x}: stack offset = {}", loc.location(), offset_str);
    }

    println!("\nEdges in the CFG:");
    for edge in cfg.graph().edge_references() {
        let source = cfg.graph().node_weight(edge.source()).unwrap();
        let target = cfg.graph().node_weight(edge.target()).unwrap();
        let op = edge.weight();
        
        let src_offset = stack_offsets.get(source)
            .map(|off| format!("{:+}", off))
            .unwrap_or_else(|| "?".to_string());
        let tgt_offset = stack_offsets.get(target)
            .map(|off| format!("{:+}", off))
            .unwrap_or_else(|| "?".to_string());
            
        println!(
            "  {:x} ({}) -> {:x} ({}): {}",
            source.location(),
            src_offset,
            target.location(),
            tgt_offset,
            op
        );
    }

    let leaf_nodes: Vec<_> = cfg.leaf_nodes().collect();
    println!("\nLeaf nodes: {} total", leaf_nodes.len());
    for leaf in &leaf_nodes {
        let offset_str = stack_offsets
            .get(leaf)
            .map(|off| format!("{:+}", off))
            .unwrap_or_else(|| "unknown".to_string());
        println!("  {:x}: stack offset = {}", leaf.location(), offset_str);
    }
}

