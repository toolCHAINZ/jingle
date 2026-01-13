use std::marker::PhantomData;

use jingle_sleigh::PcodeOperation;

use crate::analysis::cpa::{ConfigurableProgramAnalysis, state::AbstractState};

pub trait Residue<S> {
    type Output;

    /// Allows for accumulating information about a program not specific to particular abstract
    /// states.
    ///
    /// The standard CPA algorithm only accumulates program information in abstract states.
    /// However, it is often convenient to collect global program information not represented in any
    /// one state. Examples include building a CFG for the program or identifying back-edges.
    /// This method allows for implementing types to explicitly state the side-effect they would
    /// like to have on their analysis without trying to shove it into the successor iterator.
    ///
    /// This method will be called for every visited transition in the CPA, before merging. So,
    /// for every pair of states A,B visited by the CPA where A => B, this function will be called
    /// with arguments (A, B).
    ///
    /// Note that this should be used with caution if a CPA has a non-sep Merge definition; states
    /// may be refined after the CPA has made some sound effect
    fn residue(&mut self, _state: &S, _dest_state: &S, _op: &Option<PcodeOperation>) {}

    /// A hook for when two abstract states are merged.
    fn merged(
        &mut self,
        _curr_state: &S,
        _dest_state: &S,
        _merged_state: &S,
        _op: &Option<PcodeOperation>,
    ) {
    }

    fn new() -> Self;

    fn finalize(self) -> Self::Output;
}
pub struct EmptyResidue<T>(PhantomData<T>);

impl<T: AbstractState> Residue<T> for EmptyResidue<T> {
    type Output = ();
    fn new() -> Self {
        Self(Default::default())
    }

    fn finalize(self) -> Self::Output {}
}

pub struct ResidueWrapper<A: ConfigurableProgramAnalysis, R: Residue<A::State>> {
    a: A,
    _phantom: PhantomData<R>,
}

impl<A: ConfigurableProgramAnalysis, R: Residue<A::State>> ResidueWrapper<A, R> {
    pub fn wrap(a: A) -> Self {
        Self {
            a,
            _phantom: Default::default(),
        }
    }
}

impl<A: ConfigurableProgramAnalysis, R: Residue<A::State>> ConfigurableProgramAnalysis
    for ResidueWrapper<A, R>
{
    type State = A::State;

    type Reducer = R;
}
