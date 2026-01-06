#![allow(unused)]

use jingle::analysis::{Analysis, RunnableAnalysis};
use jingle::analysis::direct_location::DirectLocationAnalysis;
use jingle::analysis::stack_offset::StackOffsetAnalysis;
use jingle::analysis::pcode_store::PcodeStore;
use jingle_sleigh::context::image::gimli::load_with_gimli;
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

    // Create the compound analysis: DirectLocationAnalysis + StackOffsetAnalysis
    // DirectLocationAnalysis tracks program locations and builds a CFG
    // StackOffsetAnalysis tracks stack pointer offsets
    let location_analysis = DirectLocationAnalysis::new(&loaded);
    let stack_analysis = StackOffsetAnalysis::new(10, 100);

    let mut compound_analysis = (location_analysis, stack_analysis);

    // Run the compound analysis - now returns a tuple of both outputs
    let (cfg, stack_offsets) = compound_analysis.run(&loaded, compound_analysis.make_initial_state(FUNC_NESTED.into()));

    // Print results
    println!("Compound Analysis Results (DirectLocation + StackOffset):");
    println!("=========================================================\n");

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
        println!("  0x{:x}{}", loc, offset_info);
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

    println!("\nStack offset summary:");
    println!("  Total tracked offsets: {}", stack_offsets.len());
    if !stack_offsets.is_empty() {
        println!("  Note: Stack offset tracking requires the analysis to be");
        println!("  enhanced to properly associate offsets with program locations.");
    }
}

