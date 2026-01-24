use crate::{
    analysis::{
        Analysis,
        compound::{
            reducer::CompoundReducer, state::CompoundState, strengthen::ComponentStrengthen,
        },
        cpa::{
            ConfigurableProgramAnalysis, IntoState,
            state::{AbstractState, LocationState},
        },
    },
    modeling::machine::cpu::concrete::ConcretePcodeAddress,
};

pub mod reducer;
pub mod state;
pub mod strengthen;

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
