use jingle_sleigh::SleighArchInfo;

use crate::{
    analysis::{
        Analysis,
        cfg::{CfgState, model::StateDisplayWrapper},
        cpa::{
            ConfigurableProgramAnalysis, IntoState,
            lattice::JoinSemiLattice,
            residue::{EmptyResidue, Residue},
            state::{AbstractState, LocationState, MergeOutcome, StateDisplay},
        },
    },
    modeling::machine::cpu::concrete::ConcretePcodeAddress,
};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::OnceLock;
use std::{
    any::{Any, TypeId},
    fmt,
};
use std::{collections::HashMap, fmt::LowerHex};

type StrengthenFn = fn(&dyn Any, &dyn Any) -> Option<Box<dyn Any>>;

/// A factory wrapper used with `inventory` so each registration can provide
/// the pair `(TypeId of target, TypeId of other, StrengthenFn)` at runtime.
pub struct StrengthenFactory(pub fn() -> (TypeId, TypeId, StrengthenFn));

inventory::collect!(StrengthenFactory);

#[macro_export]
macro_rules! register_strengthen {
    ($From:ty, $To:ty, $func:path) => {
        const _: () = {
            // wrapper used to adapt the concrete fn signature `fn(&$From, &$To) -> Option<$From>`
            // into the registry-required `fn(&dyn Any, &dyn Any) -> Option<Box<dyn Any>>`.
            fn wrapper(
                a: &dyn std::any::Any,
                b: &dyn std::any::Any,
            ) -> Option<Box<dyn std::any::Any>> {
                let a = a.downcast_ref::<$From>()?;
                let b = b.downcast_ref::<$To>()?;
                ($func)(a, b).map(|r| Box::new(r) as Box<dyn std::any::Any>)
            }

            // factory function (concrete fn pointer) that returns the triple the registry expects.
            fn factory() -> (
                std::any::TypeId,
                std::any::TypeId,
                fn(&dyn std::any::Any, &dyn std::any::Any) -> Option<Box<dyn std::any::Any>>,
            ) {
                (
                    std::any::TypeId::of::<$From>(),
                    std::any::TypeId::of::<$To>(),
                    wrapper
                        as fn(
                            &dyn std::any::Any,
                            &dyn std::any::Any,
                        ) -> Option<Box<dyn std::any::Any>>,
                )
            }

            // Submit the factory to inventory so it is discovered at link time.
            inventory::submit! {
                $crate::analysis::compound::StrengthenFactory(factory)
            }
        };
    };
}

static STRENGTHEN_REGISTRY: OnceLock<HashMap<(TypeId, TypeId), StrengthenFn>> = OnceLock::new();

fn build_strengthen_registry() -> HashMap<(TypeId, TypeId), StrengthenFn> {
    let mut m = HashMap::new();
    for f in inventory::iter::<StrengthenFactory> {
        let (a, b, fun) = (f.0)();
        m.insert((a, b), fun);
    }
    m
}

fn register_lookup(src: TypeId, other: TypeId) -> Option<StrengthenFn> {
    let map = STRENGTHEN_REGISTRY.get_or_init(build_strengthen_registry);
    map.get(&(src, other)).copied()
}

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
    fn fmt_state(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    fn merge(&mut self, other: &Self) -> super::cpa::state::MergeOutcome {
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
    ) -> super::cpa::state::Successor<'a, Self> {
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

pub trait ComponentStrengthen: 'static {
    /// Attempt to strengthen `self` using information from `other`.
    ///
    /// The default implementation performs a lookup in the inventory-backed
    /// registry of strengthening functions keyed by `(TypeId of Self, TypeId of other)`.
    /// Registered functions have signature `fn(&dyn Any, &dyn Any) -> Option<Box<dyn Any>>`
    /// and are expected to return a boxed concrete `Self` on success.
    fn try_strengthen<'a, 'b>(&'a self, other: &'b dyn Any) -> Option<Self>
    where
        Self: Sized,
    {
        if let Some(func) = register_lookup(TypeId::of::<Self>(), other.type_id()) {
            if let Some(boxed) = func(self as &dyn Any, other) {
                // Attempt to downcast the returned box back to the concrete Self.
                if let Ok(concrete) = boxed.downcast::<Self>() {
                    return Some(*concrete);
                }
            }
        }
        None
    }
}

impl<T: AbstractState> ComponentStrengthen for T where T: 'static {}

pub struct CompoundReducer<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis> {
    a: A::Reducer,
    b: B::Reducer,
}

impl<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis>
    Residue<CompoundState<A::State, B::State>> for CompoundReducer<A, B>
{
    type Output = (
        <A::Reducer as Residue<A::State>>::Output,
        <B::Reducer as Residue<B::State>>::Output,
    );

    fn new() -> Self {
        Self {
            a: A::Reducer::new(),
            b: B::Reducer::new(),
        }
    }

    fn finalize(self) -> Self::Output {
        let Self { a, b } = self;
        (a.finalize(), b.finalize())
    }

    fn merged_state(
        &mut self,
        curr_state: &CompoundState<A::State, B::State>,
        dest_state: &CompoundState<A::State, B::State>,
        merged_state: &CompoundState<A::State, B::State>,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        self.a
            .merged_state(&curr_state.0, &dest_state.0, &merged_state.0, op);
        self.b
            .merged_state(&curr_state.1, &dest_state.1, &merged_state.1, op);
    }

    fn new_state(
        &mut self,
        state: &CompoundState<A::State, B::State>,
        dest_state: &CompoundState<A::State, B::State>,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        self.a.new_state(&state.0, &dest_state.0, op);
        self.b.new_state(&state.1, &dest_state.1, op);
    }
}

impl<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis> ConfigurableProgramAnalysis
    for (A, B)
where
    A::State: ComponentStrengthen,
    B::State: ComponentStrengthen,
{
    type State = CompoundState<A::State, B::State>;

    type Reducer = CompoundReducer<A, B>;
}

impl<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis, T> IntoState<(A, B)> for T
where
    T: IntoState<A> + IntoState<B> + Clone,
    A::State: 'static,
    B::State: 'static,
{
    fn into_state(self, c: &(A, B)) -> <(A, B) as ConfigurableProgramAnalysis>::State {
        // Convert the input into each component's state. We clone `self` so we can
        // consume it twice (IntoState takes self by value).
        let left = Clone::clone(&self).into_state(&c.0);
        let right = self.into_state(&c.1);
        CompoundState(left, right)
    }
}
/// Implementation of LocationState for CompoundState.
/// The location information comes from the left component.
impl<S1: LocationState, S2: AbstractState> LocationState for CompoundState<S1, S2>
where
    S1: 'static,
    S2: 'static,
{
    fn get_operation<'a, T: crate::analysis::pcode_store::PcodeStore + ?Sized>(
        &'a self,
        t: &'a T,
    ) -> Option<crate::analysis::pcode_store::PcodeOpRef<'a>> {
        self.0.get_operation(t)
    }

    fn get_location(&self) -> Option<ConcretePcodeAddress> {
        self.0.get_location()
    }
}

impl<A, B> Analysis for (A, B)
where
    A: Analysis,
    B: ConfigurableProgramAnalysis,
    A::State: LocationState + 'static,
    B::State: AbstractState + 'static,
{
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:x}, {:x})", self.0, self.1)
    }
}
