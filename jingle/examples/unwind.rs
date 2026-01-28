#![allow(unused)]

use jingle::analysis::cpa::RunnableConfigurableProgramAnalysis;
use jingle::analysis::cpa::reducer::CfgReducer;
use jingle::analysis::cpa::residue::Residue;
use jingle::analysis::cpa::state::LocationState;
use jingle::analysis::location::{CallBehavior, UnwindingAnalysis};
use jingle::analysis::{Analysis, location::BasicLocationAnalysis};
use jingle::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::context::image::gimli::load_with_gimli;
use petgraph::dot::Dot;
use std::{env, fs};

/// Addresses of various test functions in the example binary.
const FUNC_LINE: u64 = 0x100000460;
const FUNC_BRANCH: u64 = 0x100000480;
const FUNC_SWITCH: u64 = 0x1000004a0;
const FUNC_LOOP: u64 = 0x100000548;
const FUNC_NESTED: u64 = 0x100000588;
const FUNC_GOTO: u64 = 0x100000610;

fn main() {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(true)
        .init();

    tracing::info!("Starting unwinding analysis with back-edge visit counting");

    // Load binary via gimli-backed image context
    let bin_path = env::home_dir()
        .unwrap()
        .join("Documents/test_funcs/build/example");
    let loaded = load_with_gimli(bin_path, "/Applications/ghidra").unwrap();

    tracing::info!("Binary loaded successfully");

    // Run unwinding analysis - back-edges are computed internally
    tracing::info!("Running unwinding analysis with bounded back-edge visit counting");

    let location_analysis = (
        BasicLocationAnalysis::new(CallBehavior::Branch),
        UnwindingAnalysis::new(2),
    );

    // Wrap with CfgReducer
    let mut analysis_with_cfg = location_analysis.with_residue(CfgReducer::new());

    // Run the unwinding analysis
    let cfg = analysis_with_cfg.run(&loaded, ConcretePcodeAddress::from(FUNC_NESTED));

    // Print results
    println!("\nUnwinding Analysis Results:");
    println!("===========================\n");

    println!("CFG nodes (unwound states): {}", cfg.nodes().count());

    let mut locations: Vec<_> = cfg.nodes().filter_map(|n| n.get_location()).collect();
    locations.sort();
    locations.dedup();

    println!("Unique program locations: {}", locations.len());
    for loc in &locations {
        let count = cfg
            .nodes()
            .filter(|n| n.get_location() == Some(*loc))
            .count();
        println!("  0x{:x} (visited {} times)", loc, count);
    }
    fs::write("dot.dot", format!("{}", Dot::new(cfg.graph())));
    println!(
        "\nTotal CFG nodes with unwinding: {}",
        cfg.graph().node_count()
    );

    tracing::info!("Analysis complete");
}
