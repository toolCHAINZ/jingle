use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::pcode_store::PcodeStore;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;

pub enum StrengthenOutcome {
    Changed,
    Unchanged,
}

pub trait Strengthen<O: AbstractState>: AbstractState {
    fn strengthen(
        &mut self,
        _original: &(Self, O),
        _other: &O,
        _op: &PcodeOperation,
    ) -> StrengthenOutcome {
        StrengthenOutcome::Unchanged
    }
}

/// A compound analysis that combines two CPAs.
///
/// If `A` is `CompoundAnalysis<B>`, then `A` and `B` are both CPAs,
/// and `A` can be strengthened using information from `B`'s states.
///
/// # Example
///
/// ```ignore
/// // Given two CPAs where A implements Analysis and CompoundAnalysis<B>
/// struct MyCPA { /* ... */ }
/// struct AuxiliaryCPA { /* ... */ }
///
/// impl CompoundAnalysis<AuxiliaryCPA> for MyCPA {
///     // CompoundAnalysis is a marker trait with no methods
/// }
///
/// // The tuple (MyCPA, AuxiliaryCPA) will automatically implement Analysis
/// // by delegating to MyCPA's Analysis implementation
/// let compound = (my_cpa, auxiliary_cpa);
/// let result = compound.run(&store, initial_state);
/// ```
pub trait CompoundAnalysis<O: ConfigurableProgramAnalysis>: ConfigurableProgramAnalysis
where
    Self::State: Strengthen<O::State>,
{
}

/// Blanket implementation: If A's state can be strengthened by CompoundState<B::State, C::State>,
/// and (B, C) is a valid CPA, then A implements CompoundAnalysis<(B, C)>.
/// This allows nesting compound analyses.
impl<A, B, C> CompoundAnalysis<(B, C)> for A
where
    A: ConfigurableProgramAnalysis,
    B: ConfigurableProgramAnalysis,
    C: ConfigurableProgramAnalysis,
    B: CompoundAnalysis<C>,
    B::State: Strengthen<C::State>,
    A::State: Strengthen<(B::State, C::State)>,
{
}

impl<S1: AbstractState, S2: AbstractState> AbstractState for (S1, S2)
where
    S1: Strengthen<S2>,
{
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        let outcome_left = self.0.merge(&other.0);
        if outcome_left.merged() {
            self.1.merge(&other.1);
            MergeOutcome::Merged
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

    fn transfer<'a, Op: Borrow<PcodeOperation>>(&'a self, opcode: Op) -> Successor<'a, Self> {
        let opcode_ref = opcode.borrow();

        // Get successors from both analyses
        let successors_left: Vec<S1> = self.0.transfer(opcode_ref).into_iter().collect();
        let successors_right: Vec<S2> = self.1.transfer(opcode_ref).into_iter().collect();

        // Create cartesian product of successors
        let mut result = Vec::new();
        for left in successors_left {
            for right in &successors_right {
                let mut new_left = left.clone();
                new_left.strengthen(self, right, opcode_ref);
                result.push((new_left, right.clone()));
            }
        }

        result.into_iter().into()
    }
}

pub struct CompoundReducer<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis> {
    a: A::Reducer,
    b: B::Reducer,
}

impl<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis> Residue<(A::State, B::State)>
    for CompoundReducer<A, B>
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

    fn merged(
        &mut self,
        curr_state: &(A::State, B::State),
        dest_state: &(A::State, B::State),
        merged_state: &(A::State, B::State),
        op: &Option<PcodeOperation>,
    ) {
        self.a
            .merged(&curr_state.0, &dest_state.0, &merged_state.0, op);
        self.b
            .merged(&curr_state.1, &dest_state.1, &merged_state.1, op);
    }

    fn residue(
        &mut self,
        state: &(A::State, B::State),
        dest_state: &(A::State, B::State),
        op: &Option<PcodeOperation>,
    ) {
        self.a.residue(&state.0, &dest_state.0, op);
        self.b.residue(&state.1, &dest_state.1, op);
    }
}

impl<A, B> ConfigurableProgramAnalysis for (A, B)
where
    A: CompoundAnalysis<B>,
    B: ConfigurableProgramAnalysis,
    A::State: Strengthen<B::State>,
{
    type State = (A::State, B::State);
    type Reducer = CompoundReducer<A, B>;
}

impl<A: ConfigurableProgramAnalysis, B: ConfigurableProgramAnalysis, T> IntoState<(A, B)> for T
where
    A: CompoundAnalysis<B>,
    A::State: Strengthen<B::State>,
    T: IntoState<A> + IntoState<B> + Clone,
{
    fn into_state(self, c: &(A, B)) -> <(A, B) as ConfigurableProgramAnalysis>::State {
        // Convert the input into each component's state. We clone `self` so we can
        // consume it twice (IntoState takes self by value).
        let left = Clone::clone(&self).into_state(&c.0);
        let right = self.into_state(&c.1);
        (left, right)
    }
}
/// Implementation of LocationState for CompoundState.
/// The location information comes from the left component.
impl<S1: LocationState, S2: AbstractState> LocationState for (S1, S2)
where
    S1: Strengthen<S2>,
{
    fn get_operation<T: PcodeStore>(&self, t: &T) -> Option<PcodeOperation> {
        self.0.get_operation(t)
    }

    fn get_location(&self) -> Option<ConcretePcodeAddress> {
        self.0.get_location()
    }
}

/// Auto-implementation of Analysis for tuple-based compound CPAs.
/// This allows (A, B) to automatically implement Analysis when:
/// - A implements Analysis and CompoundAnalysis<B>
/// - B implements ConfigurableProgramAnalysis (may or may not implement Analysis)
/// - A::State implements Strengthen<B::State>
///
///  The output is a Vec of compound states.
impl<A, B> crate::analysis::Analysis for (A, B)
where
    A: crate::analysis::Analysis + CompoundAnalysis<B>,
    B: ConfigurableProgramAnalysis,
    A::State: Strengthen<B::State> + LocationState,
    B::State: AbstractState,
    (A::State, B::State): LocationState,
{
}

// Specialized Analysis implementation removed — the generic tuple `Analysis` implementation
// above now covers `(DirectLocationAnalysis, DirectValuationAnalysis)` and the previous
// empty/specialized impl caused a conflicting implementation error.
