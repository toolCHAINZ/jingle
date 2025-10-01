use petgraph::prelude::DiGraph;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

pub struct UnwoundLocation{
    count: usize,
    location: ConcretePcodeAddress
}

pub struct UnwoundCfg{
    max: usize,
    graph: DiGraph<UnwoundLocation, ()>
}