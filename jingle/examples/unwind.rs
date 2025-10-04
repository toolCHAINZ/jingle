#![allow(unused)]

use jingle::analysis::Analysis;
use jingle::analysis::bounded_visit::BoundedStepLocationAnalysis;
use jingle::analysis::unwinding::UnwindingAnalysis;
use jingle_sleigh::context::image::gimli::load_with_gimli;
use petgraph::dot::Dot;
use std::{env, fs};

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

    let mut direct = UnwindingAnalysis::new(1);
    let pcode_graph = direct.run(&loaded, direct.make_initial_state(FUNC_LOOP.into()));
    let addrs = pcode_graph.nodes().collect::<Vec<_>>();
    for addr in addrs {
        println!("{:x}", addr.location());
    }
    let leaf = pcode_graph.leaf_nodes().collect::<Vec<_>>();

    fs::write("dot.dot", format!("{:?}", Dot::new(&pcode_graph.graph())));
    println!("{:x?}", leaf);
    dbg!(pcode_graph.test_build(loaded.arch_info()));
}
