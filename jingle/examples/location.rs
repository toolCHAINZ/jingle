#![allow(unused)]

use jingle::analysis::bounded_branch::BoundedBranchAnalysis;
use jingle::analysis::cpa::RunnableConfigurableProgramAnalysis;
use jingle::analysis::cpa::reducer::CfgReducer;
use jingle::analysis::cpa::residue::Residue;
use jingle::analysis::cpa::state::LocationState;
use jingle::analysis::direct_location::{CallBehavior, DirectLocationAnalysis};
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

    let mut direct = (
        DirectLocationAnalysis::new(CallBehavior::Branch),
        BoundedBranchAnalysis::new(20),
    )
        .with_residue(CfgReducer::new());
    // Run the analysis. For a `ResidueWrapper` with `CfgReducer`, `run` returns
    // the built `PcodeCfg` as the reducer output, so capture it here.
    let pcode_graph = direct.run(&loaded, ConcretePcodeAddress::from(FUNC_NESTED));
    let addrs = pcode_graph.nodes().collect::<Vec<_>>();
    for node in addrs {
        // `node` is a tuple like `(DirectLocationState, BoundedBranchState)`.
        // Call `get_location` on the first element (the location-carrying component)
        // to avoid requiring trait-method resolution on the tuple itself.
        match node.0.get_location() {
            Some(a) => println!("{:?}", a),
            None => println!("(no location)"),
        }
    }
    let leaf = pcode_graph.leaf_nodes().collect::<Vec<_>>();
    for node in leaf {
        match node.0.get_location() {
            Some(a) => println!("leaf: {:?}", a),
            None => println!("leaf: (no location)"),
        }
    }
}
