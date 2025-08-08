use crate::analysis::bounded_visit::back_edge_visit_count::BackEdgeVisitCount;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::lattice::flat::FlatLattice::Value;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::cmp::Ordering;
use crate::analysis::cpa::ConfigurableProgramAnalysis;

mod back_edge_visit_count;

#[derive(Debug, Eq, PartialEq, Ord, Clone)]
struct BackEdge(ConcretePcodeAddress, ConcretePcodeAddress);

impl PartialOrd for BackEdge {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let first = self.0.cmp(&other.0);
        if first == Ordering::Equal {
            Some(other.1.cmp(&self.1))
        } else {
            Some(first)
        }
    }
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Clone)]
struct BoundedVisitState<const N: usize> {
    visits: BackEdgeVisitCount<N>,
    edges: [BackEdge; N],
    location: PcodeAddressLattice,
}

impl<const N: usize> JoinSemiLattice for BoundedVisitState<N> {
    fn join(&mut self, _other: &Self) {
        // we will not be using `Join` on this lattice
        unimplemented!()
    }
}

impl<const N: usize> AbstractState for BoundedVisitState<N> {
    type SuccessorIter = Box<dyn Iterator<Item = Self>>;

    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer(&self, opcode: &PcodeOperation) -> Self::SuccessorIter {
        let state = self.clone();
        let next_locs: Vec<_> = self.location.transfer(opcode).collect();
        let location = self.location.clone();
        let edges = self.edges.to_vec();
        Box::new(next_locs.into_iter().map(move |loc| {
            let mut state = state.clone();
            match (location, loc) {
                (Value(from), Value(to)) => {
                    if let Some(i) = edges.iter().position(|a| a == &BackEdge(from, to)) {
                        state.visits.increment(i);
                    }
                }
                _ => {}
            }
            state
        }))
    }
}

impl<const N: usize> BoundedVisitState<N>{
    pub fn new(location: ConcretePcodeAddress, edges: [BackEdge; N]) -> Self {
        Self{
            location: Value(location), edges, visits: Default::default()
        }
    }
}
struct BoundedVisitCPA<const N: usize> {
    visits: BackEdgeVisitCount<N>,
}

impl<const N: usize> ConfigurableProgramAnalysis for BoundedVisitCPA<N> {
    type State = BoundedVisitState<N>;
    type Iter = ();

    fn successor_states(&mut self, state: &Self::State) -> Self::Iter {
        todo!()
    }
}