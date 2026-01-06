use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::state::{AbstractState, MergeOutcome, Successor};
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;

pub enum StrengthenOutcome {
    Changed,
    Unchanged,
}

pub trait Strengthen<O: AbstractState>: AbstractState {
    fn strengthen(&mut self, other: &O) -> StrengthenOutcome{
        StrengthenOutcome::Unchanged
    }
}

/// A compound analysis that combines two CPAs.
///
/// If `A` is `CompoundAnalysis<B>`, then `A` and `B` are both CPAs,
/// and `A` can be strengthened using information from `B`'s states.
trait CompoundAnalysis<O: ConfigurableProgramAnalysis>: ConfigurableProgramAnalysis
where
    Self::State: Strengthen<O::State>,
{
}

/// A state that combines two abstract states from different CPAs.
#[derive(Debug, Clone)]
pub struct CompoundState<S1, S2> {
    pub left: S1,
    pub right: S2,
}

impl<S1, S2> CompoundState<S1, S2> {
    pub fn new(left: S1, right: S2) -> Self {
        Self { left, right }
    }
}

impl<S1: PartialEq, S2: PartialEq> PartialEq for CompoundState<S1, S2> {
    fn eq(&self, other: &Self) -> bool {
        self.left == other.left && self.right == other.right
    }
}

impl<S1: Eq, S2: Eq> Eq for CompoundState<S1, S2> {}

impl<S1: PartialOrd, S2: PartialOrd> PartialOrd for CompoundState<S1, S2> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (
            self.left.partial_cmp(&other.left),
            self.right.partial_cmp(&other.right),
        ) {
            (Some(Ordering::Equal), Some(Ordering::Equal)) => Some(Ordering::Equal),
            (Some(Ordering::Less), Some(Ordering::Less)) => Some(Ordering::Less),
            (Some(Ordering::Less), Some(Ordering::Equal)) => Some(Ordering::Less),
            (Some(Ordering::Equal), Some(Ordering::Less)) => Some(Ordering::Less),
            (Some(Ordering::Greater), Some(Ordering::Greater)) => Some(Ordering::Greater),
            (Some(Ordering::Greater), Some(Ordering::Equal)) => Some(Ordering::Greater),
            (Some(Ordering::Equal), Some(Ordering::Greater)) => Some(Ordering::Greater),
            _ => None,
        }
    }
}

impl<S1: JoinSemiLattice, S2: JoinSemiLattice> JoinSemiLattice for CompoundState<S1, S2> {
    fn join(&mut self, other: &Self) {
        self.left.join(&other.left);
        self.right.join(&other.right);
    }
}

impl<S1: AbstractState, S2: AbstractState> AbstractState for CompoundState<S1, S2> where S1: Strengthen<S2> {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        let outcome_left = self.left.merge(&other.left);
        let outcome_right = self.right.merge(&other.right);

        if outcome_left.merged() || outcome_right.merged() {
            MergeOutcome::Merged
        } else {
            MergeOutcome::NoOp
        }
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        // A state should stop if both components would stop
        // We need to collect states since we can't clone the iterator
        let states_vec: Vec<&Self> = states.collect();

        let stop_left = self.left.stop(states_vec.iter().map(|s| &s.left));
        let stop_right = self.right.stop(states_vec.iter().map(|s| &s.right));

        stop_left && stop_right
    }

    fn transfer<'a, Op: Borrow<PcodeOperation>>(&'a self, opcode: Op) -> Successor<'a, Self> {
        let opcode_ref = opcode.borrow();

        // Get successors from both analyses
        let successors_left: Vec<S1> = self.left.transfer(opcode_ref).into_iter().collect();
        let successors_right: Vec<S2> = self.right.transfer(opcode_ref).into_iter().collect();

        // Create cartesian product of successors
        let mut result = Vec::new();
        for left in successors_left {
            for right in &successors_right {
                let mut left = left.clone();
                left.strengthen(&right);
                result.push(CompoundState::new(left, right.clone()));
            }
        }

        result.into_iter().into()
    }
}

impl<A, B> ConfigurableProgramAnalysis for (A, B)
where
    A: CompoundAnalysis<B>,
    B: ConfigurableProgramAnalysis,
    A::State: Strengthen<B::State>,
{
    type State = CompoundState<A::State, B::State>;

    fn reduce(
        &mut self,
        state: &Self::State,
        dest_state: &Self::State,
        op: &Option<PcodeOperation>,
    ) {
        self.0.reduce(&state.left, &dest_state.left, op);
        self.1.reduce(&state.right, &dest_state.right, op);
    }

    fn merged(
        &mut self,
        curr_state: &Self::State,
        dest_state: &Self::State,
        merged_state: &Self::State,
        op: &Option<PcodeOperation>,
    ) {
        self.0
            .merged(&curr_state.left, &dest_state.left, &merged_state.left, op);
        self.1.merged(
            &curr_state.right,
            &dest_state.right,
            &merged_state.right,
            op,
        );
    }
}
