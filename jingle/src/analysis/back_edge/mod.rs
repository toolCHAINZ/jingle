use crate::analysis::Analysis;
use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use crate::analysis::direct_location::DirectLocationCPA;
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::io::empty;
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
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        self.merge_sep(other)
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        self.stop_sep(states)
    }

    fn transfer<B: Borrow<PcodeOperation>>(&self, opcode: B) -> Successor<Self> {
        let opcode = opcode.borrow();
        let s = self.clone();

        self.location
            .transfer(opcode)
            .into_iter()
            .map(|a| self.add_location(a))
            .into()
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

    fn successor_states<'a>(&self, state: &'a Self::State) -> Successor<'a, Self::State> {
        match self.location.pcode_at(&state.location) {
            Some(op) => state.transfer(&op).into_iter().into(),
            None => std::iter::empty().into(),
        }
    }

    fn reduce(&mut self, old_state: &Self::State, new_state: &Self::State) {
        if old_state.path_visits.contains(&new_state.location) {
            self.back_edges
                .push((old_state.location.clone(), new_state.location.clone()))
        }
    }
}

pub struct BackEdgeAnalysis;

impl Analysis for BackEdgeAnalysis {
    type Output = HashMap<ConcretePcodeAddress, ConcretePcodeAddress>;
    type Input = BackEdgeState;

    fn run<T: PcodeStore, I: Into<Self::Input>>(
        &mut self,
        store: T,
        initial_state: I,
    ) -> Self::Output {
        let initial_state = initial_state.into();
        let mut cpa = BackEdgeCPA::new(store);
        let _ = cpa.run_cpa(initial_state);
        cpa.back_edges
            .into_iter()
            .filter_map(|(a, b)| a.value().and_then(|av| b.value().map(|bv| (*av, *bv))))
            .collect()
    }

    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input {
        BackEdgeState::new(PcodeAddressLattice::Value(addr))
    }
}
