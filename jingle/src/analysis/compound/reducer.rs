use crate::analysis::{
    compound::state::{CompoundState2, CompoundState3, CompoundState4},
    cpa::{ConfigurableProgramAnalysis, residue::Residue},
};

/// Generic 2-ary compound reducer (keeps the old name for backward compatibility).
pub struct CompoundReducer<'op, A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis> {
    a: A::Reducer<'op>,
    b: B::Reducer<'op>,
}

impl<'op, A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis>
    Residue<'op, CompoundState2<A::State, B::State>> for CompoundReducer<'op, A, B>
{
    type Output = (
        <A::Reducer<'op> as Residue<'op, A::State>>::Output,
        <B::Reducer<'op> as Residue<'op, B::State>>::Output,
    );

    fn new() -> Self {
        Self {
            a: <A::Reducer<'op> as Residue<'op, A::State>>::new(),
            b: <B::Reducer<'op> as Residue<'op, B::State>>::new(),
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
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
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
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
    ) {
        self.a.new_state(&state.s1, &dest_state.s1, op);
        self.b.new_state(&state.s2, &dest_state.s2, op);
    }
}

/// Explicitly named 2-ary reducer.
pub struct CompoundReducer2<'op, A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis> {
    a: A::Reducer<'op>,
    b: B::Reducer<'op>,
}

impl<'op, A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis>
    Residue<'op, CompoundState2<A::State, B::State>> for CompoundReducer2<'op, A, B>
{
    type Output = (
        <A::Reducer<'op> as Residue<'op, A::State>>::Output,
        <B::Reducer<'op> as Residue<'op, B::State>>::Output,
    );

    fn new() -> Self {
        Self {
            a: <A::Reducer<'op> as Residue<'op, A::State>>::new(),
            b: <B::Reducer<'op> as Residue<'op, B::State>>::new(),
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
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
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
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
    ) {
        self.a.new_state(&state.s1, &dest_state.s1, op);
        self.b.new_state(&state.s2, &dest_state.s2, op);
    }
}

/// 3-ary compound reducer.
pub struct CompoundReducer3<
    'op,
    A: ConfigurableProgramAnalysis,
    B: ConfigurableProgramAnalysis,
    C: ConfigurableProgramAnalysis,
> {
    a: A::Reducer<'op>,
    b: B::Reducer<'op>,
    c: C::Reducer<'op>,
}

impl<
    'op,
    A: ConfigurableProgramAnalysis,
    B: ConfigurableProgramAnalysis,
    C: ConfigurableProgramAnalysis,
> Residue<'op, CompoundState3<A::State, B::State, C::State>> for CompoundReducer3<'op, A, B, C>
{
    type Output = (
        <A::Reducer<'op> as Residue<'op, A::State>>::Output,
        <B::Reducer<'op> as Residue<'op, B::State>>::Output,
        <C::Reducer<'op> as Residue<'op, C::State>>::Output,
    );

    fn new() -> Self {
        Self {
            a: <A::Reducer<'op> as Residue<'op, A::State>>::new(),
            b: <B::Reducer<'op> as Residue<'op, B::State>>::new(),
            c: <C::Reducer<'op> as Residue<'op, C::State>>::new(),
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
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
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
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
    ) {
        self.a.new_state(&state.s1, &dest_state.s1, op);
        self.b.new_state(&state.s2, &dest_state.s2, op);
        self.c.new_state(&state.s3, &dest_state.s3, op);
    }
}

/// 4-ary compound reducer.
pub struct CompoundReducer4<
    'op,
    A: ConfigurableProgramAnalysis,
    B: ConfigurableProgramAnalysis,
    C: ConfigurableProgramAnalysis,
    D: ConfigurableProgramAnalysis,
> {
    a: A::Reducer<'op>,
    b: B::Reducer<'op>,
    c: C::Reducer<'op>,
    d: D::Reducer<'op>,
}

impl<
    'op,
    A: ConfigurableProgramAnalysis,
    B: ConfigurableProgramAnalysis,
    C: ConfigurableProgramAnalysis,
    D: ConfigurableProgramAnalysis,
> Residue<'op, CompoundState4<A::State, B::State, C::State, D::State>>
    for CompoundReducer4<'op, A, B, C, D>
{
    type Output = (
        <A::Reducer<'op> as Residue<'op, A::State>>::Output,
        <B::Reducer<'op> as Residue<'op, B::State>>::Output,
        <C::Reducer<'op> as Residue<'op, C::State>>::Output,
        <D::Reducer<'op> as Residue<'op, D::State>>::Output,
    );

    fn new() -> Self {
        Self {
            a: <A::Reducer<'op> as Residue<'op, A::State>>::new(),
            b: <B::Reducer<'op> as Residue<'op, B::State>>::new(),
            c: <C::Reducer<'op> as Residue<'op, C::State>>::new(),
            d: <D::Reducer<'op> as Residue<'op, D::State>>::new(),
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
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
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
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
    ) {
        self.a.new_state(&state.s1, &dest_state.s1, op);
        self.b.new_state(&state.s2, &dest_state.s2, op);
        self.c.new_state(&state.s3, &dest_state.s3, op);
        self.d.new_state(&state.s4, &dest_state.s4, op);
    }
}
