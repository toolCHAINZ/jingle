use crate::analysis::{
    compound::state::CompoundState2,
    cpa::{ConfigurableProgramAnalysis, residue::Residue},
};

pub struct CompoundReducer<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis> {
    a: A::Reducer,
    b: B::Reducer,
}

impl<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis>
    Residue<CompoundState2<A::State, B::State>> for CompoundReducer<A, B>
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
        curr_state: &CompoundState2<A::State, B::State>,
        dest_state: &CompoundState2<A::State, B::State>,
        merged_state: &CompoundState2<A::State, B::State>,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        self.a
            .merged_state(&curr_state.0, &dest_state.0, &merged_state.0, op);
        self.b
            .merged_state(&curr_state.1, &dest_state.1, &merged_state.1, op);
    }

    fn new_state(
        &mut self,
        state: &CompoundState2<A::State, B::State>,
        dest_state: &CompoundState2<A::State, B::State>,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        self.a.new_state(&state.0, &dest_state.0, op);
        self.b.new_state(&state.1, &dest_state.1, op);
    }
}
