use crate::{
    analysis::{
        Analysis,
        compound::{
            reducer::{CompoundReducer, CompoundReducer2, CompoundReducer3, CompoundReducer4},
            state::{CompoundState2, CompoundState3, CompoundState4},
            strengthen::ComponentStrengthen,
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

// 2-tuple (existing)
impl<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis> ConfigurableProgramAnalysis
    for (A, B)
where
    A::State: ComponentStrengthen,
    B::State: ComponentStrengthen,
{
    type State = CompoundState2<A::State, B::State>;

    type Reducer = CompoundReducer2<A, B>;
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
        CompoundState2 {
            s1: left,
            s2: right,
        }
    }
}

// 3-tuple: use nested reducer CompoundReducer<(A,B), C>
impl<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis, C: ConfigurableProgramAnalysis>
    ConfigurableProgramAnalysis for (A, B, C)
where
    A::State: ComponentStrengthen,
    B::State: ComponentStrengthen,
    C::State: ComponentStrengthen,
{
    type State = CompoundState3<A::State, B::State, C::State>;

    // 3-ary reducer
    type Reducer = CompoundReducer3<A, B, C>;
}

impl<
    A: ConfigurableProgramAnalysis,
    B: ConfigurableProgramAnalysis,
    C: ConfigurableProgramAnalysis,
    T,
> IntoState<(A, B, C)> for T
where
    T: IntoState<A> + IntoState<B> + IntoState<C> + Clone,
    A::State: 'static,
    B::State: 'static,
    C::State: 'static,
{
    fn into_state(self, c: &(A, B, C)) -> <(A, B, C) as ConfigurableProgramAnalysis>::State {
        // We need to produce three component states; clone `self` as needed.
        let left = Clone::clone(&self).into_state(&c.0);
        let middle = Clone::clone(&self).into_state(&c.1);
        let right = self.into_state(&c.2);
        CompoundState3 {
            s1: left,
            s2: middle,
            s3: right,
        }
    }
}

// 4-tuple: nested reducers ((A,B),C),D
impl<
    A: ConfigurableProgramAnalysis,
    B: ConfigurableProgramAnalysis,
    C: ConfigurableProgramAnalysis,
    D: ConfigurableProgramAnalysis,
> ConfigurableProgramAnalysis for (A, B, C, D)
where
    A::State: ComponentStrengthen,
    B::State: ComponentStrengthen,
    C::State: ComponentStrengthen,
    D::State: ComponentStrengthen,
{
    type State = CompoundState4<A::State, B::State, C::State, D::State>;

    // 4-ary reducer
    type Reducer = CompoundReducer4<A, B, C, D>;
}

impl<
    A: ConfigurableProgramAnalysis,
    B: ConfigurableProgramAnalysis,
    C: ConfigurableProgramAnalysis,
    D: ConfigurableProgramAnalysis,
    T,
> IntoState<(A, B, C, D)> for T
where
    T: IntoState<A> + IntoState<B> + IntoState<C> + IntoState<D> + Clone,
    A::State: 'static,
    B::State: 'static,
    C::State: 'static,
    D::State: 'static,
{
    fn into_state(self, c: &(A, B, C, D)) -> <(A, B, C, D) as ConfigurableProgramAnalysis>::State {
        // Produce four component states; clone `self` as needed.
        let s0 = Clone::clone(&self);
        let s1 = Clone::clone(&self);
        let s2 = Clone::clone(&self);
        let left = s0.into_state(&c.0);
        let s_middle = s1.into_state(&c.1);
        let s_next = s2.into_state(&c.2);
        let right = self.into_state(&c.3);
        CompoundState4 {
            s1: left,
            s2: s_middle,
            s3: s_next,
            s4: right,
        }
    }
}
