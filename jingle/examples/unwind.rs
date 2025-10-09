#![allow(unused)]

use jingle::analysis::Analysis;
use jingle::analysis::bounded_visit::BoundedStepLocationAnalysis;
use jingle::analysis::unwinding::UnwindingAnalysis;
use jingle_sleigh::context::image::gimli::load_with_gimli;
use petgraph::dot::Dot;
use std::time::Instant;
use std::{env, fs};
use z3::{Config, Params, with_z3_config};

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
    z3::set_global_param("trace", "true");
    let loaded = load_with_gimli(bin_path, "/Applications/ghidra").unwrap();

    let mut direct = UnwindingAnalysis::new(2);
    let pcode_graph = direct.run(&loaded, direct.make_initial_state(0x1000004a0.into()));
    let addrs = pcode_graph.nodes().collect::<Vec<_>>();
    for addr in addrs {
        println!("{:x}", addr.location());
    }
    let leaf = pcode_graph.leaf_nodes().collect::<Vec<_>>();
    let w = pcode_graph.edge_weights().collect::<Vec<_>>();

    fs::write("dot.dot", format!("{:x}", Dot::new(&pcode_graph.graph())));
    println!("{:x?}", leaf);
    let arch_info = loaded.arch_info();;
    with_z3_config(&Config::new(), || {
        let solver = pcode_graph.test_build(arch_info);
        let mut params = Params::new();
        params.set_bool("trace", true);
        solver.set_params(&params);
        fs::write(
            "test.smt",
            pcode_graph.test_build(arch_info).to_string(),
        );
        let t = Instant::now();
        dbg!(solver.check());
        dbg!(solver.get_unsat_core());
        println!("took {:?}", t.elapsed());
    })
}
