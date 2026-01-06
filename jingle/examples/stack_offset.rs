#![allow(unused)]

use jingle::analysis::{Analysis, RunnableAnalysis};
use jingle::analysis::direct_location::DirectLocationAnalysis;
use jingle::analysis::direct_valuation::{DirectValuationAnalysis, DirectValuationState, VarnodeValue};
use jingle::analysis::pcode_store::PcodeStore;
use jingle_sleigh::context::image::gimli::load_with_gimli;
use jingle_sleigh::VarNode;
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

    tracing::info!("Starting stack offset analysis using DirectValuationAnalysis");

    let bin_path = env::home_dir()
        .unwrap()
        .join("Documents/test_funcs/build/example");
    let loaded = load_with_gimli(bin_path, "/Applications/ghidra").unwrap();

    tracing::info!("Binary loaded successfully");

    // Create the stack pointer varnode (RSP on x86-64)
    let stack_pointer = VarNode {
        space_index: 4, // Register space
        offset: 8,      // RSP offset on x86-64
        size: 8,        // 8 bytes for 64-bit
    };

    // Create a compound analysis: DirectLocationAnalysis + DirectValuationAnalysis
    // DirectValuationAnalysis will track the stack pointer as an Entry value
    let location_analysis = DirectLocationAnalysis::new(&loaded);
    let valuation_analysis = DirectValuationAnalysis::with_entry_varnode(stack_pointer.clone());

    let mut compound_analysis = (location_analysis, valuation_analysis);

    tracing::info!("Starting analysis run at address 0x{:x}", FUNC_NESTED);

    // Run the compound analysis - returns Vec of compound states
    let compound_states = compound_analysis.run(&loaded, compound_analysis.make_initial_state(FUNC_NESTED.into()));

    tracing::info!("Analysis completed with {} states", compound_states.len());

    // Extract the CFG from the DirectLocationAnalysis (left side of compound)
    let cfg = compound_analysis.0.take_cfg();

    // Extract valuation information from the compound states
    use std::collections::HashMap;
    use jingle::analysis::compound::CompoundState;
    let mut stack_offsets = HashMap::new();
    let mut direct_valuations = HashMap::new();

    for state in &compound_states {
        // Extract location from the outermost left (PcodeAddressLattice)
        if let jingle::analysis::cpa::lattice::flat::FlatLattice::Value(addr) = &state.left {
            // state.right is DirectValuationState
            // Extract stack pointer offset if available
            if let Some(sp_value) = state.right.get_value(&stack_pointer) {
                stack_offsets.insert(*addr, sp_value.clone());
            }
            direct_valuations.insert(*addr, state.right.clone());
        }
    }

    // Print results
    println!("Stack Offset Analysis Results using DirectValuationAnalysis:");
    println!("=============================================================\n");

    // Collect and sort locations for consistent output
    let mut locations = cfg.nodes().collect::<Vec<_>>();
    locations.sort();

    println!("CFG nodes (program locations): {}", locations.len());
    for loc in &locations {
        let offset_info = stack_offsets
            .get(loc)
            .map(|value| match value {
                VarnodeValue::Entry(_) => " [stack: Entry (0)]".to_string(),
                VarnodeValue::Offset(_, off) => format!(" [stack: {:+}]", off),
                VarnodeValue::Const(c) => format!(" [stack: const 0x{:x}]", c),
                VarnodeValue::Top => " [stack: unknown]".to_string(),
                VarnodeValue::Bottom => " [stack: bottom]".to_string(),
            })
            .unwrap_or_default();

        // Show a sample of tracked constant values at this location
        let val_count = direct_valuations
            .get(loc)
            .map(|v: &DirectValuationState| v.written_locations().len())
            .unwrap_or(0);
        let val_info = if val_count > 0 {
            format!(" [tracked varnodes: {}]", val_count)
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
            .map(|value| match value {
                VarnodeValue::Entry(_) => " [stack: Entry (0)]".to_string(),
                VarnodeValue::Offset(_, off) => format!(" [stack: {:+}]", off),
                VarnodeValue::Const(c) => format!(" [stack: const 0x{:x}]", c),
                VarnodeValue::Top => " [stack: unknown]".to_string(),
                VarnodeValue::Bottom => " [stack: bottom]".to_string(),
            })
            .unwrap_or_default();
        println!("  0x{:x}{}", leaf, offset_info);
    }

    println!("\nAnalysis Summary:");
    println!("  Total program locations: {}", stack_offsets.len());

    // Count how many locations have concrete stack offsets
    let concrete_offsets = stack_offsets
        .values()
        .filter(|v| matches!(v, VarnodeValue::Entry(_) | VarnodeValue::Offset(_, _)))
        .count();

    println!("  Concrete stack offsets: {}", concrete_offsets);
    println!("  Total tracked varnodes across all locations: {}",
        direct_valuations.values().map(|v: &DirectValuationState| v.written_locations().len()).sum::<usize>());

    println!("\n  DirectValuationAnalysis acts as a lightweight pcode interpreter that tracks");
    println!("  all directly-written varnodes. The stack pointer is initialized as an Entry");
    println!("  value, and the analysis tracks how it changes through the program as");
    println!("  Offset(sp, delta) values. This replaces the need for a separate");
    println!("  StackOffsetAnalysis.");
}

