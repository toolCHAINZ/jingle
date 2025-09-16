use crate::analysis::Analysis;
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome};
use crate::analysis::direct_location::DirectLocationCPA;
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::vec::IntoIter;

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct BackEdgeState {
    pub(crate) path_visits: HashSet<PcodeAddressLattice>,
    pub(crate) location: PcodeAddressLattice,
}

impl BackEdgeState {
    pub fn new(location: PcodeAddressLattice) -> BackEdgeState {
        Self {
            location,
            path_visits: Default::default(),
        }
    }
    pub fn add_location(&self, loc: PcodeAddressLattice) -> BackEdgeState {
        let mut s = self.clone();
        s.location = loc;
        s.path_visits.insert(loc);
        s
    }
}
impl PartialOrd for BackEdgeState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.location.value() == other.location.value() {
            let other_visits = other.path_visits.get(&other.location)?;
            let self_visits = self.path_visits.get(&self.location)?;
            if self_visits == other_visits {
                Some(Ordering::Equal)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl JoinSemiLattice for BackEdgeState {
    fn join(&mut self, _other: &Self) {
        // We don't use `join` on this state so no need to implement it
        unimplemented!()
    }
}

impl AbstractState for BackEdgeState {
    type SuccessorIter = Box<dyn Iterator<Item = BackEdgeState>>;

    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer(&self, opcode: &PcodeOperation) -> Self::SuccessorIter {
        let s = self.clone();
        Box::new(
            self.location
                .transfer(opcode)
                .map(move |a| s.add_location(a)),
        )
    }
}

struct BackEdgeCPA<T: PcodeStore> {
    location: DirectLocationCPA<T>,
    pub back_edges: Vec<(PcodeAddressLattice, PcodeAddressLattice)>,
}

impl<T: PcodeStore> BackEdgeCPA<T> {
    pub fn new(pcode: T) -> Self {
        Self {
            location: DirectLocationCPA::new(pcode),
            back_edges: Vec::new(),
        }
    }
}

impl<T: PcodeStore> ConfigurableProgramAnalysis for BackEdgeCPA<T> {
    type State = BackEdgeState;
    type Iter = IntoIter<BackEdgeState>;

    fn successor_states(&mut self, state: &Self::State) -> Self::Iter {
        let state = state.clone();
        let o: Vec<_> = match self.location.pcode_at(&state.location) {
            Some(op) => state
                .transfer(&op)
                .inspect(|a| {
                    if state.path_visits.contains(&a.location) {
                        self.back_edges.push((state.location, a.location));
                    }
                })
                .collect(),
            None => vec![],
        };
        o.into_iter()
    }
}

pub struct BackEdgeAnalysis;

impl Analysis for BackEdgeAnalysis {
    type Output = HashMap<ConcretePcodeAddress, ConcretePcodeAddress>;
    type Input = BackEdgeState;

    fn run<T: PcodeStore, I: Into<Self::Input>>(&mut self, store: T, initial_state: I) -> Self::Output {
        let initial_state = initial_state.into();
        let mut cpa = BackEdgeCPA::new(store);
        let _ = cpa.run_cpa(&initial_state);
        cpa.back_edges
            .into_iter()
            .filter_map(|(a, b)| a.value().and_then(|av| b.value().map(|bv| (*av, *bv))))
            .collect()
    }

    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input {
        BackEdgeState::new(PcodeAddressLattice::Value(addr))
    }
}
