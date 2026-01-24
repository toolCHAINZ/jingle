use crate::analysis::cpa::{
    ConfigurableProgramAnalysis,
    lattice::JoinSemiLattice,
    residue::EmptyResidue,
    state::{AbstractState, StateDisplay},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundState<S1, S2>(S1, S2);

impl<S1: PartialOrd, S2: PartialOrd> PartialOrd for CompoundState<S1, S2> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.0.partial_cmp(&other.0) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.1.partial_cmp(&other.1)
    }
}

impl<S1: JoinSemiLattice, S2: JoinSemiLattice> JoinSemiLattice for CompoundState<S1, S2> {
    fn join(&mut self, other: &Self) {
        todo!()
    }
}

impl<S1, S2> StateDisplay for CompoundState<S1, S2> {
    fn fmt_state(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<S1: AbstractState + ComponentStrengthen, S2: AbstractState + ComponentStrengthen> AbstractState
    for CompoundState<S1, S2>
{
    fn merge(&mut self, other: &Self) -> super::cpa::state::MergeOutcome {
        todo!()
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        todo!()
    }

    fn transfer<'a, B: std::borrow::Borrow<jingle_sleigh::PcodeOperation>>(
        &'a self,
        opcode: B,
    ) -> super::cpa::state::Successor<'a, Self> {
        self.0.try_strengthen(&self.1);
        self.1.try_strengthen(&self.0);
        todo!()
    }
}

pub trait ComponentStrengthen {
    fn try_strengthen(&self, other: &impl AbstractState) -> Option<Self>
    where
        Self: Sized,
    {
        None
    }
}

impl<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis> ConfigurableProgramAnalysis
    for (A, B)
where
    A::State: ComponentStrengthen,
    B::State: ComponentStrengthen,
{
    type State = CompoundState<A::State, B::State>;

    type Reducer = EmptyResidue<Self::State>;
}
