#![allow(unused)]

use jingle::analysis::Analysis;
use jingle::analysis::bounded_visit::BoundedBranchAnalysis;
use jingle::analysis::cfg::PcodeCfgVisitor;
use jingle::analysis::ctl::*;
use jingle::analysis::unwinding::{UnwindingAnalysis, UnwoundLocation};
use jingle::modeling::machine::MachineState;
use jingle::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle::modeling::machine::memory::MemoryState;
use jingle_sleigh::PcodeOperation;
use jingle_sleigh::context::image::gimli::load_with_gimli;
use petgraph::dot::Dot;
use std::time::Instant;
use std::{env, fs};
use z3::ast::Bool;
use z3::{Config, Params, Solver, with_z3_config};

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

    let mut direct = UnwindingAnalysis::new_with_bounds(&loaded, 10);
    let _states =
        direct.run_with_back_edges(&loaded, direct.make_initial_state(FUNC_NESTED.into()));
    let pcode_graph = direct.take_cfg();
    let pcode_graph = pcode_graph.smt_model();
    // let pcode_graph = pcode_graph.basic_blocks();
    let addrs = pcode_graph.nodes().collect::<Vec<_>>();
    for addr in addrs {
        println!("{:x}", addr.location());
    }
    let leaf = pcode_graph.leaf_nodes().collect::<Vec<_>>();
    let w = pcode_graph.edge_weights().collect::<Vec<_>>();

    fs::write("dot.dot", format!("{:x}", Dot::new(&pcode_graph.graph())));
    let ctl_model = EF(CtlFormula::proposition(
        |a: &PcodeCfgVisitor<UnwoundLocation>, b: Option<&PcodeOperation>| {
            a.state()
                .unwrap()
                .pc()
                .eq(&ConcretePcodeAddress::from(0x100000604).symbolize())
        },
    ));
    let state = pcode_graph
        .nodes_for_location(ConcretePcodeAddress::from(FUNC_NESTED))
        .next()
        .unwrap();
    let check = pcode_graph.check_model(state, ctl_model);
    let solver = Solver::new();
    solver.assert(check);
    println!("check");
    dbg!(solver.check());
    dbg!(solver.get_model());
    dbg!(solver.get_unsat_core());
    //let arch_info = loaded.arch_info();
    //let solver = pcode_graph.test_build(arch_info);
    //let mut params = Params::new();
    //params.set_bool("trace", true);
    //solver.set_params(&params);
    //fs::write("test.smt", pcode_graph.test_build(arch_info).to_string());
    //let t = Instant::now();
    //dbg!(solver.check());
    //dbg!(solver.get_unsat_core());
    //println!("took {:?}", t.elapsed());
}
