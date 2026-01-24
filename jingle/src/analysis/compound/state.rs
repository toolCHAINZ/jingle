use jingle_sleigh::SleighArchInfo;
use std::fmt::Debug;
use std::hash::Hash;
use std::{any::Any, fmt::LowerHex};

use crate::{
    analysis::{
        cfg::{CfgState, model::StateDisplayWrapper},
        compound::strengthen::ComponentStrengthen,
        cpa::{
            lattice::JoinSemiLattice,
            state::{AbstractState, MergeOutcome, StateDisplay, Successor},
        },
    },
    modeling::machine::cpu::concrete::ConcretePcodeAddress,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompoundState<S1, S2>(pub S1, pub S2);

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
        self.0.join(&other.0);
        self.1.join(&other.1);
    }
}

impl<S1: StateDisplay, S2: StateDisplay> StateDisplay for CompoundState<S1, S2> {
    fn fmt_state(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(")?;
        self.0.fmt_state(f)?;
        write!(f, ", ")?;
        self.1.fmt_state(f)?;
        write!(f, ")")
    }
}

impl<S1: AbstractState + ComponentStrengthen, S2: AbstractState + ComponentStrengthen> AbstractState
    for CompoundState<S1, S2>
{
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        let outcome_left = self.0.merge(&other.0);
        if outcome_left.merged() || self.0 == other.0 {
            self.1.merge(&other.1)
        } else {
            MergeOutcome::NoOp
        }
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        // A state should stop if both components would stop
        // We need to collect states since we can't clone the iterator
        let states_vec: Vec<&Self> = states.collect();

        let stop_left = self.0.stop(states_vec.iter().map(|s| &s.0));
        let stop_right = self.1.stop(states_vec.iter().map(|s| &s.1));
        stop_left && stop_right
    }

    fn transfer<'a, B: std::borrow::Borrow<jingle_sleigh::PcodeOperation>>(
        &'a self,
        opcode: B,
    ) -> Successor<'a, Self> {
        let opcode_ref = opcode.borrow();

        // Get successors from both analyses
        let successors_left: Vec<S1> = self.0.transfer(opcode_ref).into_iter().collect();
        let successors_right: Vec<S2> = self.1.transfer(opcode_ref).into_iter().collect();

        // Create cartesian product of successors
        let mut result = Vec::new();
        for left in &successors_left {
            for right in &successors_right {
                let new_left = left
                    .try_strengthen(right as &dyn Any)
                    .unwrap_or(left.clone());
                let new_right = right
                    .try_strengthen(left as &dyn Any)
                    .unwrap_or(right.clone());
                result.push(CompoundState(new_left, new_right));
            }
        }

        result.into_iter().into()
    }
}

impl<A: CfgState, B: StateDisplay + Clone + Debug + Hash + Eq> CfgState for CompoundState<A, B> {
    type Model = A::Model;

    fn new_const(&self, i: &SleighArchInfo) -> Self::Model {
        self.0.new_const(i)
    }

    fn model_id(&self) -> String {
        // Incorporate the display output from the second element into the model id.
        // Use an underscore separator to keep ids readable and safe.
        format!("{}_{}", self.0.model_id(), StateDisplayWrapper(&self.1))
    }

    fn location(&self) -> Option<ConcretePcodeAddress> {
        self.0.location()
    }
}

impl<S1: LowerHex, S2: LowerHex> LowerHex for CompoundState<S1, S2> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:x}, {:x})", self.0, self.1)
    }
}
