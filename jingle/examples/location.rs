#![allow(unused)]

use jingle::analysis::bounded_visit::BoundedStepLocationAnalysis;
use jingle::analysis::{Analysis, RunnableAnalysis};
use jingle::modeling::machine::cpu::concrete::ConcretePcodeAddress;
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

    let mut direct = BoundedStepLocationAnalysis::new(&loaded, 20);
    let _states = direct.run(&loaded, ConcretePcodeAddress::from(FUNC_NESTED));
    let pcode_graph = direct.take_cfg();
    let addrs = pcode_graph.nodes().collect::<Vec<_>>();
    for addr in addrs {
        println!("{:x}", addr);
    }
    let leaf = pcode_graph.leaf_nodes().collect::<Vec<_>>();
    println!("{:x?}", leaf);
}
