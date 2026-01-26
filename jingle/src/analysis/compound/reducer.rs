use crate::analysis::{
    compound::state::{CompoundState2, CompoundState3, CompoundState4},
    cpa::{ConfigurableProgramAnalysis, residue::Residue},
};

/// Generic 2-ary compound reducer (keeps the old name for backward compatibility).
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
            .merged_state(&curr_state.s1, &dest_state.s1, &merged_state.s1, op);
        self.b
            .merged_state(&curr_state.s2, &dest_state.s2, &merged_state.s2, op);
    }

    fn new_state(
        &mut self,
        state: &CompoundState2<A::State, B::State>,
        dest_state: &CompoundState2<A::State, B::State>,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        self.a.new_state(&state.s1, &dest_state.s1, op);
        self.b.new_state(&state.s2, &dest_state.s2, op);
    }
}

/// Explicitly named 2-ary reducer.
pub struct CompoundReducer2<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis> {
    a: A::Reducer,
    b: B::Reducer,
}

impl<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis>
    Residue<CompoundState2<A::State, B::State>> for CompoundReducer2<A, B>
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
            .merged_state(&curr_state.s1, &dest_state.s1, &merged_state.s1, op);
        self.b
            .merged_state(&curr_state.s2, &dest_state.s2, &merged_state.s2, op);
    }

    fn new_state(
        &mut self,
        state: &CompoundState2<A::State, B::State>,
        dest_state: &CompoundState2<A::State, B::State>,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        self.a.new_state(&state.s1, &dest_state.s1, op);
        self.b.new_state(&state.s2, &dest_state.s2, op);
    }
}

/// 3-ary compound reducer.
pub struct CompoundReducer3<
    A: ConfigurableProgramAnalysis,
    B: ConfigurableProgramAnalysis,
    C: ConfigurableProgramAnalysis,
> {
    a: A::Reducer,
    b: B::Reducer,
    c: C::Reducer,
}

impl<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis, C: ConfigurableProgramAnalysis>
    Residue<CompoundState3<A::State, B::State, C::State>> for CompoundReducer3<A, B, C>
{
    type Output = (
        <A::Reducer as Residue<A::State>>::Output,
        <B::Reducer as Residue<B::State>>::Output,
        <C::Reducer as Residue<C::State>>::Output,
    );

    fn new() -> Self {
        Self {
            a: A::Reducer::new(),
            b: B::Reducer::new(),
            c: C::Reducer::new(),
        }
    }

    fn finalize(self) -> Self::Output {
        let Self { a, b, c } = self;
        (a.finalize(), b.finalize(), c.finalize())
    }

    fn merged_state(
        &mut self,
        curr_state: &CompoundState3<A::State, B::State, C::State>,
        dest_state: &CompoundState3<A::State, B::State, C::State>,
        merged_state: &CompoundState3<A::State, B::State, C::State>,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        self.a
            .merged_state(&curr_state.s1, &dest_state.s1, &merged_state.s1, op);
        self.b
            .merged_state(&curr_state.s2, &dest_state.s2, &merged_state.s2, op);
        self.c
            .merged_state(&curr_state.s3, &dest_state.s3, &merged_state.s3, op);
    }

    fn new_state(
        &mut self,
        state: &CompoundState3<A::State, B::State, C::State>,
        dest_state: &CompoundState3<A::State, B::State, C::State>,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        self.a.new_state(&state.s1, &dest_state.s1, op);
        self.b.new_state(&state.s2, &dest_state.s2, op);
        self.c.new_state(&state.s3, &dest_state.s3, op);
    }
}

/// 4-ary compound reducer.
pub struct CompoundReducer4<
    A: ConfigurableProgramAnalysis,
    B: ConfigurableProgramAnalysis,
    C: ConfigurableProgramAnalysis,
    D: ConfigurableProgramAnalysis,
> {
    a: A::Reducer,
    b: B::Reducer,
    c: C::Reducer,
    d: D::Reducer,
}

impl<
    A: ConfigurableProgramAnalysis,
    B: ConfigurableProgramAnalysis,
    C: ConfigurableProgramAnalysis,
    D: ConfigurableProgramAnalysis,
> Residue<CompoundState4<A::State, B::State, C::State, D::State>> for CompoundReducer4<A, B, C, D>
{
    type Output = (
        <A::Reducer as Residue<A::State>>::Output,
        <B::Reducer as Residue<B::State>>::Output,
        <C::Reducer as Residue<C::State>>::Output,
        <D::Reducer as Residue<D::State>>::Output,
    );

    fn new() -> Self {
        Self {
            a: A::Reducer::new(),
            b: B::Reducer::new(),
            c: C::Reducer::new(),
            d: D::Reducer::new(),
        }
    }

    fn finalize(self) -> Self::Output {
        let Self { a, b, c, d } = self;
        (a.finalize(), b.finalize(), c.finalize(), d.finalize())
    }

    fn merged_state(
        &mut self,
        curr_state: &CompoundState4<A::State, B::State, C::State, D::State>,
        dest_state: &CompoundState4<A::State, B::State, C::State, D::State>,
        merged_state: &CompoundState4<A::State, B::State, C::State, D::State>,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        self.a
            .merged_state(&curr_state.s1, &dest_state.s1, &merged_state.s1, op);
        self.b
            .merged_state(&curr_state.s2, &dest_state.s2, &merged_state.s2, op);
        self.c
            .merged_state(&curr_state.s3, &dest_state.s3, &merged_state.s3, op);
        self.d
            .merged_state(&curr_state.s4, &dest_state.s4, &merged_state.s4, op);
    }

    fn new_state(
        &mut self,
        state: &CompoundState4<A::State, B::State, C::State, D::State>,
        dest_state: &CompoundState4<A::State, B::State, C::State, D::State>,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'_>>,
    ) {
        self.a.new_state(&state.s1, &dest_state.s1, op);
        self.b.new_state(&state.s2, &dest_state.s2, op);
        self.c.new_state(&state.s3, &dest_state.s3, op);
        self.d.new_state(&state.s4, &dest_state.s4, op);
    }
}
