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
                    let mut state = $name{$($field),+};
                    state.do_strengthen();
                    state
                }).into()
            }
        }
    };

}

named_tuple!(CompoundState2, s1: S1, s2: S2);
named_tuple!(CompoundState3, s1: S1, s2: S2, s3: S3);
named_tuple!(CompoundState4, s1: S1, s2: S2, s3: S3, s4: S4);

impl<S1: ComponentStrengthen, S2: ComponentStrengthen> CompoundState2<S1, S2> {
    fn do_strengthen(&mut self) {
        self.s1.try_strengthen(&self.s2);
        self.s2.try_strengthen(&self.s1);
    }
}

impl<S1: ComponentStrengthen, S2: ComponentStrengthen, S3: ComponentStrengthen>
    CompoundState3<S1, S2, S3>
{
    fn do_strengthen(&mut self) {
        self.s1.try_strengthen(&self.s2);
        self.s1.try_strengthen(&self.s3);

        self.s2.try_strengthen(&self.s1);
        self.s2.try_strengthen(&self.s3);

        self.s3.try_strengthen(&self.s1);
        self.s3.try_strengthen(&self.s2);
    }
}

impl<
    S1: ComponentStrengthen,
    S2: ComponentStrengthen,
    S3: ComponentStrengthen,
    S4: ComponentStrengthen,
> CompoundState4<S1, S2, S3, S4>
{
    fn do_strengthen(&mut self) {
        self.s1.try_strengthen(&self.s2);
        self.s1.try_strengthen(&self.s3);
        self.s1.try_strengthen(&self.s4);

        self.s2.try_strengthen(&self.s1);
        self.s2.try_strengthen(&self.s3);
        self.s2.try_strengthen(&self.s4);

        self.s3.try_strengthen(&self.s1);
        self.s3.try_strengthen(&self.s2);
        self.s3.try_strengthen(&self.s4);

        self.s4.try_strengthen(&self.s1);
        self.s4.try_strengthen(&self.s2);
        self.s4.try_strengthen(&self.s3);
    }
}

impl<A: CfgState, B: StateDisplay + Clone + Debug + Hash + Eq> CfgState for CompoundState2<A, B> {
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

impl<S1: LowerHex, S2: LowerHex> LowerHex for CompoundState2<S1, S2> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:x}, {:x})", self.s1, self.s2)
    }
}
