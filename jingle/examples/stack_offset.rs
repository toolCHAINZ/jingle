#![allow(unused)]

use jingle::analysis::{Analysis, RunnableAnalysis};
use jingle::analysis::direct_location::DirectLocationAnalysis;
use jingle::analysis::direct_valuation::{DirectValuationAnalysis, DirectValuationState};
use jingle::analysis::stack_offset::StackOffsetAnalysis;
use jingle::analysis::pcode_store::PcodeStore;
use jingle_sleigh::context::image::gimli::load_with_gimli;
use std::env;
use tracing_subscriber;

const FUNC_LINE: u64 = 0x100000460;
const FUNC_BRANCH: u64 = 0x100000480;
const FUNC_SWITCH: u64 = 0x1000004a0;
const FUNC_LOOP: u64 = 0x100000548;
const FUNC_NESTED: u64 = 0x100000588;
const FUNC_GOTO: u64 = 0x100000610;

fn main() {
    // Initialize tracing with TRACE level
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(true)
        .init();

    tracing::info!("Starting compound analysis example with direct valuation strengthening");

    let bin_path = env::home_dir()
        .unwrap()
        .join("Documents/test_funcs/build/example");
    let loaded = load_with_gimli(bin_path, "/Applications/ghidra").unwrap();

    tracing::info!("Binary loaded successfully");

    // Create a compound analysis: (DirectLocationAnalysis + StackOffsetAnalysis) strengthened by DirectValuationAnalysis
    // This demonstrates how direct valuation can refine stack offset analysis
    //
    // The analysis pipeline:
    // 1. DirectLocationAnalysis tracks program locations and builds a CFG
    // 2. StackOffsetAnalysis tracks stack pointer offsets
    // 3. DirectValuationAnalysis tracks constant writes to varnodes
    // 4. StackOffsetAnalysis is strengthened by DirectValuationAnalysis to use known constants

    let location_analysis = DirectLocationAnalysis::new(&loaded);
    let stack_and_valuation = (StackOffsetAnalysis, DirectValuationAnalysis);

    let mut compound_analysis = (location_analysis, stack_and_valuation);

    tracing::info!("Starting compound analysis run at address 0x{:x}", FUNC_NESTED);

    // Run the compound analysis - returns Vec of compound states
    let compound_states = compound_analysis.run(&loaded, compound_analysis.make_initial_state(FUNC_NESTED.into()));

    tracing::info!("Analysis completed with {} states", compound_states.len());

    // Extract the CFG from the DirectLocationAnalysis (left side of compound)
    let cfg = compound_analysis.0.take_cfg();

    // Extract stack offset and valuation information from the compound states
    use std::collections::HashMap;
    use jingle::analysis::compound::CompoundState;
    let mut stack_offsets = HashMap::new();
    let mut direct_valuations = HashMap::new();

    for state in &compound_states {
        // Extract location from the outermost left (PcodeAddressLattice)
        if let jingle::analysis::cpa::lattice::flat::FlatLattice::Value(addr) = &state.left {
            // state.right is CompoundState<StackOffsetState, DirectValuationState>
            stack_offsets.insert(*addr, state.right.left.offset().clone());
            direct_valuations.insert(*addr, state.right.right.clone());
        }
    }

    // Print results
    println!("Compound Analysis Results (DirectLocation + (StackOffset strengthened by DirectValuation)):");
    println!("========================================================================================\n");

    // Collect and sort locations for consistent output
    let mut locations = cfg.nodes().collect::<Vec<_>>();
    locations.sort();

    println!("CFG nodes (program locations): {}", locations.len());
    for loc in &locations {
        let offset_info = stack_offsets
            .get(loc)
            .map(|offset| match offset {
                jingle::analysis::stack_offset::StackOffsetLattice::Offset(v) => format!(" [stack: {:+}]", v),
                jingle::analysis::stack_offset::StackOffsetLattice::Range(min, max) => format!(" [stack: {:+}..{:+}]", min, max),
                jingle::analysis::stack_offset::StackOffsetLattice::Top => " [stack: unknown]".to_string(),
                jingle::analysis::stack_offset::StackOffsetLattice::Bottom => " [stack: bottom]".to_string(),
            })
            .unwrap_or_default();

        // Show a sample of tracked constant values at this location
        let val_count = direct_valuations
            .get(loc)
            .map(|v: &DirectValuationState| v.written_locations().len())
            .unwrap_or(0);
        let val_info = if val_count > 0 {
            format!(" [tracked constants: {}]", val_count)
        } else {
            String::new()
        };

        println!("  0x{:x}{}{}", loc, offset_info, val_info);
    }

    println!("\nCFG edges:");
    let nodes = cfg.nodes().collect::<Vec<_>>();
    for node in nodes {
        if let Some(successors) = cfg.successors(node) {
            for succ in successors {
                let op_str = cfg.get_op_at(node)
                    .map(|o: &jingle_sleigh::PcodeOperation| format!("{}", o))
                    .unwrap_or_else(|| "no-op".to_string());
                println!("  0x{:x} -> 0x{:x}: {}", node, succ, op_str);
            }
        }
    }

    let leaf_nodes = cfg.leaf_nodes().collect::<Vec<_>>();
    println!("\nLeaf nodes: {}", leaf_nodes.len());
    for leaf in &leaf_nodes {
        let offset_info = stack_offsets
            .get(leaf)
            .map(|offset| match offset {
                jingle::analysis::stack_offset::StackOffsetLattice::Offset(v) => format!(" [stack: {:+}]", v),
                jingle::analysis::stack_offset::StackOffsetLattice::Range(min, max) => format!(" [stack: {:+}..{:+}]", min, max),
                jingle::analysis::stack_offset::StackOffsetLattice::Top => " [stack: unknown]".to_string(),
                jingle::analysis::stack_offset::StackOffsetLattice::Bottom => " [stack: bottom]".to_string(),
            })
            .unwrap_or_default();
        println!("  0x{:x}{}", leaf, offset_info);
    }

    println!("\nAnalysis Summary:");
    println!("  Total program locations: {}", stack_offsets.len());

    // Count how many locations have concrete stack offsets
    let concrete_offsets = stack_offsets
        .values()
        .filter(|o| matches!(o, jingle::analysis::stack_offset::StackOffsetLattice::Offset(_)))
        .count();

    println!("  Concrete stack offsets: {}", concrete_offsets);
    println!("  Total tracked constant values across all locations: {}",
        direct_valuations.values().map(|v: &DirectValuationState| v.written_locations().len()).sum::<usize>());

    println!("\n  The StackOffsetAnalysis is strengthened by DirectValuationAnalysis,");
    println!("  which means when a stack operation uses a varnode with a known constant");
    println!("  value (tracked by DirectValuation), that constant is used to refine the");
    println!("  stack offset calculation, potentially resulting in more precise offsets.");
}

