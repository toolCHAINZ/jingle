use crate::analysis::cpa::state::LocationState;
use crate::{
    analysis::{
        cfg::CfgState,
        compound::strengthen::ComponentStrengthen,
        cpa::{
            lattice::JoinSemiLattice,
            state::{AbstractState, MergeOutcome, Successor},
        },
    },
    modeling::machine::cpu::concrete::ConcretePcodeAddress,
};
use itertools::iproduct;
use jingle_sleigh::SleighArchInfo;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::LowerHex;
use std::hash::Hash;

macro_rules! named_tuple {
    // capture: struct name, first field, then repeated `ident: TypeIdent`
    ( $name:ident, $first_field:ident : $F:ident, $( $field:ident : $T:ident ),+ $(,)? ) => {
        // declare the struct with generics
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name<$F, $( $T ),+ > {
            pub $first_field: $F,
            $( pub $field: $T ),+
        }

        impl<$F: PartialOrd, $( $T: PartialOrd ),+> PartialOrd for $name<$F, $( $T ),+> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                // Lexicographic comparison: compare fields in declaration order,
                // returning the first non-Equal ordering. If any component's
                // partial_cmp returns None, propagate None.
                let mut curr = self.$first_field.partial_cmp(&other.$first_field)?;

                $(
                    let next = self.$field.partial_cmp(&other.$field)?;
                    if curr == Ordering::Equal{
                        curr = next;
                    }else{
                        if(curr != next && next != Ordering::Equal){
                            return None;
                        }
                    }
                )+
                Some(curr)
            }
        }

        impl<$F: JoinSemiLattice, $( $T: JoinSemiLattice ),+> JoinSemiLattice for $name<$F, $( $T ),+> {
            fn join(&mut self, other: &Self) {
                self.$first_field.join(&other.$first_field);
                $(
                    self.$field.join(&other.$field);
                )+
            }
        }

        impl<$F: Display, $( $T: Display ),+> Display for $name<$F, $( $T ),+> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "(")?;
                self.$first_field.fmt(f)?;
                $(
                    write!(f, ",")?;
                    self.$field.fmt(f)?;
                )+
                write!(f, ")")
            }
        }



        impl<$F: ComponentStrengthen + AbstractState, $( $T: ComponentStrengthen + AbstractState ),+> AbstractState
            for $name<$F, $( $T ),+>
        {
            fn merge(&mut self, other: &Self) -> MergeOutcome {
                let mut res = MergeOutcome::NoOp;
                let eq = self.$first_field == other.$first_field;
                res += self.$first_field.merge(&other.$first_field);
                if !eq && res == MergeOutcome::NoOp{
                    return res;
                }

                $(
                    let eq = self.$field == other.$field;
                    res += self.$field.merge(&other.$field);
                    if !eq && res == MergeOutcome::NoOp{
                        return res;
                    }
                )+
                res
            }

            fn stop<'a, I: Iterator<Item = &'a Self>>(&'a self, states: I) -> bool {
                // todo: maybe should allow customizing this?
                self.stop_sep(states)
            }

            fn transfer<'a, B: std::borrow::Borrow<jingle_sleigh::PcodeOperation>>(
                &'a self,
                opcode: B,
            ) -> Successor<'a, Self> {
                let opcode_ref = opcode.borrow();
                iproduct!(
                    self.$first_field.transfer(opcode_ref).into_iter()
                    $(, self.$field.transfer(opcode_ref).into_iter() )+
                ).map(|( first $(, $field )+ )| {
                    // destructure names from the tuple into the struct fields
                    let mut state = $name { $first_field: first, $( $field ),+ };
                    state.do_strengthen();
                    state
                }).into()
            }
        }

        // CfgState implementation: use the first component for model and location.
        impl<$F: CfgState, $( $T: Display + Clone + Debug + Hash + Eq ),+> CfgState for $name<$F, $( $T ),+> {
            type Model = $F::Model;

            fn new_const(&self, i: &SleighArchInfo) -> Self::Model {
                self.$first_field.new_const(i)
            }

            fn model_id(&self) -> String {
                // Start with the first component's model id and append
                // StateDisplayWrapper forms for the remaining components.
                let mut id = self.$first_field.model_id();
                $(
                    id = format!("{}_{}", id, &self.$field);
                )+
                id
            }

            fn location(&self) -> Option<ConcretePcodeAddress> {
                self.$first_field.location()
            }
        }

        // LowerHex implementation: print each component in hex
        impl<$F: LowerHex, $( $T: LowerHex ),+> LowerHex for $name<$F, $( $T ),+> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "(")?;
                write!(f, "{:x}", self.$first_field)?;
                $(
                    write!(f, ", {:x}", self.$field)?;
                )+
                write!(f, ")")
            }
        }

        /// Implementation of LocationState for CompoundState.
        /// The location information comes from the first (left-most) component.
        impl<$F: LocationState, $( $T: AbstractState ),+> LocationState for $name<$F, $( $T ),+>
        where
            $F: 'static,
            $( $T: 'static ),+
        {
            fn get_operation<'a, P: crate::analysis::pcode_store::PcodeStore + ?Sized>(
                &self,
                t: &'a P,
            ) -> Option<crate::analysis::pcode_store::PcodeOpRef<'a>> {
                self.$first_field.get_operation(t)
            }

            fn get_location(&self) -> Option<ConcretePcodeAddress> {
                self.$first_field.get_location()
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
