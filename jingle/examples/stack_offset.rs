#![allow(unused)]

use jingle::analysis::Analysis;
use jingle::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use jingle::analysis::cpa::reducer::CFG;
use jingle::analysis::cpa::residue::Residue;
use jingle::analysis::cpa::state::LocationState;
use jingle::analysis::cpa::{FinalReducer, RunnableConfigurableProgramAnalysis};
use jingle::analysis::location::{BasicLocationAnalysis, CallBehavior};
use jingle::analysis::pcode_store::PcodeStore;
use jingle::analysis::pcode_store::{self, PcodeOpRef};
use jingle::analysis::valuation::{
    MergeBehavior, SimpleValuation, SimpleValuationAnalysis, SimpleValuationState,
};
use jingle::display::JingleDisplayable;
use jingle::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::VarNode;
use jingle_sleigh::context::image::gimli::load_with_gimli;
use std::collections::HashMap;
use std::env;

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
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(true)
        .init();

    tracing::info!("Starting stack offset analysis using DirectValuationAnalysis");

    // Load binary via gimli-backed image context (adjust paths to your setup)
    let bin_path = env::home_dir()
        .unwrap()
        .join("Documents/test_funcs/build/example");
    let loaded = load_with_gimli(bin_path, "/Applications/ghidra").unwrap();

    tracing::info!("Binary loaded successfully");

    // Create the stack pointer varnode (RSP on x86-64)
    let stack_pointer = VarNode {
        space_index: 4, // Register space index for registers (depends on sleigh description)
        offset: 8,      // RSP offset in the register space for this target
        size: 8,        // 8 bytes for 64-bit
    };

    // Build a compound analysis: DirectLocationAnalysis (left) + DirectValuationAnalysis (right).
    // Wrap the compound with a CfgReducer so `run` returns the constructed CFG.
    let location_analysis = BasicLocationAnalysis::new(CallBehavior::Branch);
    let valuation_analysis =
        SimpleValuationAnalysis::new(loaded.arch_info().clone(), MergeBehavior::Or);

    // The tuple implements Analysis via the compound machinery; wrap it with the CfgReducer (factory)
    let mut compound_with_cfg = (location_analysis, valuation_analysis).with_residue(CFG);

    tracing::info!("Starting analysis run at address 0x{:x}", FUNC_NESTED);

    // Run the analysis. The Residue/CfgReducer final output is a `PcodeCfg<(DirectLocationState, DirectValuationState)>`
    let cfg = compound_with_cfg.run(&loaded, ConcretePcodeAddress::from(FUNC_NESTED));

    // We'll collect valuation info keyed by concrete addresses encountered in the CFG.
    let mut stack_offsets: HashMap<ConcretePcodeAddress, SimpleValuation> = HashMap::new();
    let mut direct_valuations: HashMap<ConcretePcodeAddress, SimpleValuationState> = HashMap::new();

    // `cfg.nodes()` yields `&N` where N = (DirectLocationState, DirectValuationState).
    // Use `cloned()` to get owned tuples so we can inspect and store values.
    for node in cfg.nodes().cloned() {
        // Extract the concrete program location (if any) from the left component.
        if let Some(addr) = node.s1.get_location() {
            // Extract stack pointer info from the DirectValuationState (right component).
            if let Some(sp_value) = node.s2.get_value(&stack_pointer) {
                stack_offsets.insert(addr, sp_value.clone());
            }
            direct_valuations.insert(addr, node.s2.clone());
        }
    }

    // Print summary header
    println!("Stack Offset Analysis Results using DirectValuationAnalysis:");
    println!("=============================================================\n");

    // List CFG nodes (program locations)
    let mut locations: Vec<ConcretePcodeAddress> =
        cfg.nodes().filter_map(|n| n.get_location()).collect();
    locations.sort_by_key(|a| *a);

    println!("CFG nodes (program locations): {}", locations.len());
    for loc in &locations {
        let offset_info = stack_offsets
            .get(loc)
            .map(|value| format!("{}", value.display(loaded.arch_info())))
            .unwrap_or_default();

        let val_count = direct_valuations
            .get(loc)
            .map(|v: &SimpleValuationState| v.written_locations().len())
            .unwrap_or(0);

        let val_info = if val_count > 0 {
            format!(" [tracked varnodes: {}]", val_count)
        } else {
            String::new()
        };

        println!("  0x{:x}{}{}", loc, offset_info, val_info);
    }

    // Print CFG edges: show edges between concrete locations when available
    println!("\nCFG edges:");
    // Collect references to nodes so we can call `cfg.successors(...)`
    let node_refs = cfg.nodes().collect::<Vec<_>>();
    for node in node_refs {
        let origin_str = node
            .get_location()
            .map(|a| format!("0x{:x}", a))
            .unwrap_or_else(|| "(no loc)".to_string());

        if let Some(successors) = cfg.successors(node) {
            for succ in successors {
                let succ_str = succ
                    .get_location()
                    .map(|a| format!("0x{:x}", a))
                    .unwrap_or_else(|| "(no loc)".to_string());

                let op_str = cfg
                    .get_op_at(node)
                    .map(|o: &PcodeOpRef<'_>| format!("{}", o.as_ref()))
                    .unwrap_or_else(|| "no-op".to_string());

                println!("  {} -> {}: {}", origin_str, succ_str, op_str);
            }
        }
    }

    // Leaf nodes (nodes with no outgoing edges)
    let leaf_nodes = cfg.leaf_nodes().collect::<Vec<_>>();
    println!("\nLeaf nodes: {}", leaf_nodes.len());
    for leaf in &leaf_nodes {
        let leaf_loc = leaf
            .get_location()
            .map(|a| format!("0x{:x}", a))
            .unwrap_or_else(|| "(no loc)".to_string());
        let offset_info = leaf
            .get_location()
            .and_then(|a| stack_offsets.get(&a))
            .map(|value| format!("{}", value.display(loaded.arch_info())))
            .unwrap_or_default();

        println!("  {} {}", leaf_loc, offset_info);

        // Print detailed DirectValuationState for this leaf (if available)
        if let Some(addr) = leaf.get_location() {
            if let Some(state) = direct_valuations.get(&addr) {
                let count = state.written_locations().len();
                println!("    DirectValuationState: {} tracked varnode(s)", count);

                if count == 0 {
                    println!("      (no written locations)");
                } else {
                    for (vn, val) in state.written_locations().iter() {
                        println!(
                            "      {} = {}",
                            vn.display(loaded.arch_info()),
                            val.display(loaded.arch_info())
                        );
                    }
                }
            } else {
                println!("    (no DirectValuationState recorded for this location)");
            }
        } else {
            println!(
                "    Computed loc: {}",
                leaf.s1.inner().display(loaded.arch_info())
            );
            println!("      Valuations:");
            for ele in leaf.s2.written_locations().iter() {
                println!(
                    "        {} = {}",
                    ele.0.display(loaded.arch_info()),
                    ele.1.display(loaded.arch_info())
                )
            }
        }
    }

    // Final summary statistics
    println!("\nAnalysis Summary:");
    println!("  Total program locations: {}", stack_offsets.len());

    let concrete_offsets = stack_offsets
        .values()
        .filter(|v| !matches!(v, SimpleValuation::Top))
        .count();

    println!("  Concrete stack offsets: {}", concrete_offsets);
    println!(
        "  Total tracked varnodes across all locations: {}",
        direct_valuations
            .values()
            .map(|v: &SimpleValuationState| v.written_locations().len())
            .sum::<usize>()
    );

    println!("\n  Notes:");
    println!("  - `DirectValuationAnalysis` acts as a lightweight p-code interpreter that tracks");
    println!("    directly-written varnodes. The stack pointer can be seeded as an Entry value");
    println!(
        "    and the analysis will track `Offset(sp, delta)` values as the program modifies it."
    );
    println!("  - This example demonstrates how to run a compound analysis and extract both the");
    println!("    constructed CFG (via `CfgReducer`) and per-location valuation information.");
}
