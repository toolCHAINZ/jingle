use itertools::iproduct;
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

macro_rules! named_tuple {
    // capture: struct name, then repeated `ident: TypeIdent`
    ( $name:ident, $( $field:ident : $T:ident ),+ $(,)? ) => {
        // declare the struct with generics
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name< $( $T ),+ > {
            $( pub $field: $T ),+
        }

        impl<$($T: PartialOrd),+> PartialOrd for $name<$($T),+> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                use std::cmp::Ordering;
                // Lexicographic comparison: compare fields in declaration order,
                // returning the first non-Equal ordering. If any component's
                // partial_cmp returns None, propagate None.
                let mut res: Option<Option<Ordering>> = None;
                $(
                    match res{
                        None => res = Some(self.$field.partial_cmp(&other.$field)),
                        Some(v) => {
                            let new_res = self.$field.partial_cmp(&other.$field);
                            if  new_res != v{
                                return None;
                            }
                        }
                    }
                )+
                res.flatten()
            }
        }

        impl<$($T: JoinSemiLattice),+> JoinSemiLattice for $name<$($T),+> {
            fn join(&mut self, other: &Self) {
                $(
                    self.$field.join(&other.$field);
                )+
            }
        }

        impl<$($T: StateDisplay),+> StateDisplay for $name<$($T),+> {
            fn fmt_state(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "(")?;
                $(
                    self.$field.fmt_state(f)?;
                    write!(f, ", ")?;
                )+
                write!(f, ")")
            }
        }



        impl<$($T: ComponentStrengthen + AbstractState),+> AbstractState
            for $name<$($T),+>
        {
            fn merge(&mut self, other: &Self) -> MergeOutcome {
                    let mut overall_outcome = MergeOutcome::NoOp;
                $(
                    let outcome = self.$field.merge(&other.$field);
                    if outcome == MergeOutcome::NoOp{
                        return overall_outcome;
                    }else{
                        overall_outcome = outcome;
                    }
                )+
                overall_outcome
            }

            fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
                // A state should stop if both components would stop
                // We need to collect states since we can't clone the iterator
                let states_vec: Vec<&Self> = states.collect();
                let mut res = true;
                $(
                    res &= self.$field.stop(states_vec.iter().map(|s| &s.$field));
                )+
                res
            }

            fn transfer<'a, B: std::borrow::Borrow<jingle_sleigh::PcodeOperation>>(
                &'a self,
                opcode: B,
            ) -> Successor<'a, Self> {
                let opcode_ref = opcode.borrow();

                iproduct!($(
                    self.$field.transfer(opcode_ref).into_iter()
                ),+).map(|($($field),+)| {

                    $name{$($field),+}
                }).into()
            }
        }
    };

}

named_tuple!(CompoundState, s1: S1, s2: S2);

impl<A: CfgState, B: StateDisplay + Clone + Debug + Hash + Eq> CfgState for CompoundState<A, B> {
    type Model = A::Model;

    fn new_const(&self, i: &SleighArchInfo) -> Self::Model {
        self.s1.new_const(i)
    }

    fn model_id(&self) -> String {
        // Incorporate the display output from the second element into the model id.
        // Use an underscore separator to keep ids readable and safe.
        format!("{}_{}", self.s1.model_id(), StateDisplayWrapper(&self.s2))
    }

    fn location(&self) -> Option<ConcretePcodeAddress> {
        self.s1.location()
    }
}

impl<S1: LowerHex, S2: LowerHex> LowerHex for CompoundState<S1, S2> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:x}, {:x})", self.s1, self.s2)
    }
}
