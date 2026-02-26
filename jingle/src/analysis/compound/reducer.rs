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

    fn finalize(self, reached: Vec<CompoundState2<A::State, B::State>>) -> Self::Output {
        let Self { a, b } = self;
        // Project compound states into separate component vectors
        let a_reached: Vec<A::State> = reached.iter().map(|cs| cs.s1.clone()).collect();
        let b_reached: Vec<B::State> = reached.iter().map(|cs| cs.s2.clone()).collect();
        (a.finalize(a_reached), b.finalize(b_reached))
    }

    fn merged_state(
        &mut self,
        source_idx: usize,
        merged_idx: usize,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
    ) {
        self.a.merged_state(source_idx, merged_idx, op);
        self.b.merged_state(source_idx, merged_idx, op);
    }

    fn new_state(
        &mut self,
        source_idx: usize,
        dest_idx: usize,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
    ) {
        self.a.new_state(source_idx, dest_idx, op);
        self.b.new_state(source_idx, dest_idx, op);
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

    fn finalize(self, reached: Vec<CompoundState2<A::State, B::State>>) -> Self::Output {
        let Self { a, b } = self;
        // Project compound states into separate component vectors
        let a_reached: Vec<A::State> = reached.iter().map(|cs| cs.s1.clone()).collect();
        let b_reached: Vec<B::State> = reached.iter().map(|cs| cs.s2.clone()).collect();
        (a.finalize(a_reached), b.finalize(b_reached))
    }

    fn merged_state(
        &mut self,
        source_idx: usize,
        merged_idx: usize,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
    ) {
        self.a.merged_state(source_idx, merged_idx, op);
        self.b.merged_state(source_idx, merged_idx, op);
    }

    fn new_state(
        &mut self,
        source_idx: usize,
        dest_idx: usize,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
    ) {
        self.a.new_state(source_idx, dest_idx, op);
        self.b.new_state(source_idx, dest_idx, op);
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

    fn finalize(self, reached: Vec<CompoundState3<A::State, B::State, C::State>>) -> Self::Output {
        let Self { a, b, c } = self;
        // Project compound states into separate component vectors
        let a_reached: Vec<A::State> = reached.iter().map(|cs| cs.s1.clone()).collect();
        let b_reached: Vec<B::State> = reached.iter().map(|cs| cs.s2.clone()).collect();
        let c_reached: Vec<C::State> = reached.iter().map(|cs| cs.s3.clone()).collect();
        (a.finalize(a_reached), b.finalize(b_reached), c.finalize(c_reached))
    }

    fn merged_state(
        &mut self,
        source_idx: usize,
        merged_idx: usize,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
    ) {
        self.a.merged_state(source_idx, merged_idx, op);
        self.b.merged_state(source_idx, merged_idx, op);
        self.c.merged_state(source_idx, merged_idx, op);
    }

    fn new_state(
        &mut self,
        source_idx: usize,
        dest_idx: usize,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
    ) {
        self.a.new_state(source_idx, dest_idx, op);
        self.b.new_state(source_idx, dest_idx, op);
        self.c.new_state(source_idx, dest_idx, op);
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

    fn finalize(self, reached: Vec<CompoundState4<A::State, B::State, C::State, D::State>>) -> Self::Output {
        let Self { a, b, c, d } = self;
        // Project compound states into separate component vectors
        let a_reached: Vec<A::State> = reached.iter().map(|cs| cs.s1.clone()).collect();
        let b_reached: Vec<B::State> = reached.iter().map(|cs| cs.s2.clone()).collect();
        let c_reached: Vec<C::State> = reached.iter().map(|cs| cs.s3.clone()).collect();
        let d_reached: Vec<D::State> = reached.iter().map(|cs| cs.s4.clone()).collect();
        (a.finalize(a_reached), b.finalize(b_reached), c.finalize(c_reached), d.finalize(d_reached))
    }

    fn merged_state(
        &mut self,
        source_idx: usize,
        merged_idx: usize,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
    ) {
        self.a.merged_state(source_idx, merged_idx, op);
        self.b.merged_state(source_idx, merged_idx, op);
        self.c.merged_state(source_idx, merged_idx, op);
        self.d.merged_state(source_idx, merged_idx, op);
    }

    fn new_state(
        &mut self,
        source_idx: usize,
        dest_idx: usize,
        op: &Option<crate::analysis::pcode_store::PcodeOpRef<'op>>,
    ) {
        self.a.new_state(source_idx, dest_idx, op);
        self.b.new_state(source_idx, dest_idx, op);
        self.c.new_state(source_idx, dest_idx, op);
        self.d.new_state(source_idx, dest_idx, op);
    }
}
