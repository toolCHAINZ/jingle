use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::cpa::lattice::JoinSemiLattice;
use crate::analysis::cpa::state::{AbstractState, LocationState, MergeOutcome, Successor};
use crate::analysis::pcode_store::PcodeStore;
use jingle_sleigh::PcodeOperation;
use std::borrow::Borrow;
use std::cmp::Ordering;

pub enum StrengthenOutcome {
    Changed,
    Unchanged,
}

pub trait Strengthen<O: AbstractState>: AbstractState {
    fn strengthen(&mut self, _original: &Self, _other: &O, _op: &PcodeOperation) -> StrengthenOutcome{
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
    A::State: Strengthen<CompoundState<B::State, C::State>>,
{
}

/// A state that combines two abstract states from different CPAs.
#[derive(Debug, Clone)]
pub struct CompoundState<S1, S2> {
    pub left: S1,
    pub right: S2,
}

impl<S1, S2> CompoundState<S1, S2> {
    pub fn new(left: S1, right: S2) -> Self {
        Self { left, right }
    }
}

impl<S1: PartialEq, S2: PartialEq> PartialEq for CompoundState<S1, S2> {
    fn eq(&self, other: &Self) -> bool {
        self.left == other.left && self.right == other.right
    }
}

impl<S1: Eq, S2: Eq> Eq for CompoundState<S1, S2> {}

impl<S1: PartialOrd, S2: PartialOrd> PartialOrd for CompoundState<S1, S2> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (
            self.left.partial_cmp(&other.left),
            self.right.partial_cmp(&other.right),
        ) {
            (Some(Ordering::Equal), Some(Ordering::Equal)) => Some(Ordering::Equal),
            (Some(Ordering::Less), Some(Ordering::Less)) => Some(Ordering::Less),
            (Some(Ordering::Less), Some(Ordering::Equal)) => Some(Ordering::Less),
            (Some(Ordering::Equal), Some(Ordering::Less)) => Some(Ordering::Less),
            (Some(Ordering::Greater), Some(Ordering::Greater)) => Some(Ordering::Greater),
            (Some(Ordering::Greater), Some(Ordering::Equal)) => Some(Ordering::Greater),
            (Some(Ordering::Equal), Some(Ordering::Greater)) => Some(Ordering::Greater),
            _ => None,
        }
    }
}

impl<S1: JoinSemiLattice, S2: JoinSemiLattice> JoinSemiLattice for CompoundState<S1, S2> {
    fn join(&mut self, other: &Self) {
        self.left.join(&other.left);
        self.right.join(&other.right);
    }
}

impl<S1: AbstractState, S2: AbstractState> AbstractState for CompoundState<S1, S2> where S1: Strengthen<S2> {
    fn merge(&mut self, other: &Self) -> MergeOutcome {
        let outcome_left = self.left.merge(&other.left);
        if outcome_left.merged(){
            self.right.merge(&other.right);
            MergeOutcome::Merged
        }else {
            MergeOutcome::NoOp
        }
    }

    fn stop<'a, T: Iterator<Item = &'a Self>>(&'a self, states: T) -> bool {
        // A state should stop if both components would stop
        // We need to collect states since we can't clone the iterator
        let states_vec: Vec<&Self> = states.collect();

        let stop_left = self.left.stop(states_vec.iter().map(|s| &s.left));
        let stop_right = self.right.stop(states_vec.iter().map(|s| &s.right));
        stop_left && stop_right
    }

    fn transfer<'a, Op: Borrow<PcodeOperation>>(&'a self, opcode: Op) -> Successor<'a, Self> {
        let opcode_ref = opcode.borrow();

        // Get successors from both analyses
        let successors_left: Vec<S1> = self.left.transfer(opcode_ref).into_iter().collect();
        let successors_right: Vec<S2> = self.right.transfer(opcode_ref).into_iter().collect();

        // Create cartesian product of successors
        let mut result = Vec::new();
        for left in successors_left {
            for right in &successors_right {
                let mut new_left = left.clone();
                new_left.strengthen(&left, right, opcode_ref);
                result.push(CompoundState::new(new_left, right.clone()));
            }
        }

        result.into_iter().into()
    }
}

impl<A, B> ConfigurableProgramAnalysis for (A, B)
where
    A: CompoundAnalysis<B>,
    B: ConfigurableProgramAnalysis,
    A::State: Strengthen<B::State>,
{
    type State = CompoundState<A::State, B::State>;

    fn reduce(
        &mut self,
        state: &Self::State,
        dest_state: &Self::State,
        op: &Option<PcodeOperation>,
    ) {
        self.0.reduce(&state.left, &dest_state.left, op);
        self.1.reduce(&state.right, &dest_state.right, op);
    }

    fn merged(
        &mut self,
        curr_state: &Self::State,
        dest_state: &Self::State,
        merged_state: &Self::State,
        op: &Option<PcodeOperation>,
    ) {
        self.0
            .merged(&curr_state.left, &dest_state.left, &merged_state.left, op);
        self.1.merged(
            &curr_state.right,
            &dest_state.right,
            &merged_state.right,
            op,
        );
    }
}

/// Implementation of LocationState for CompoundState.
/// The location information comes from the left component.
impl<S1: LocationState, S2: AbstractState> LocationState for CompoundState<S1, S2> where S1: Strengthen<S2> {
    fn get_operation<T: PcodeStore>(&self, t: &T) -> Option<PcodeOperation> {
        self.left.get_operation(t)
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
    CompoundState<A::State, B::State>: LocationState,
    A::Input: Into<A::State>,
    CompoundState<A::State, B::State>: From<A::Input>,
{
    type Input = A::Input;

    fn make_initial_state(&self, addr: crate::modeling::machine::cpu::concrete::ConcretePcodeAddress) -> Self::Input {
        self.0.make_initial_state(addr)
    }

    fn make_output(&mut self, states: Vec<Self::State>) -> Vec<Self::State> {
        // Just return the compound states as-is
        // Consumers can extract left/right components as needed
        states
    }
}

// Custom Analysis implementation for DirectLocationAnalysis + DirectValuationAnalysis
// This is needed because the DirectValuationAnalysis needs to initialize its entry varnode
impl crate::analysis::Analysis for (crate::analysis::direct_location::DirectLocationAnalysis, crate::analysis::direct_valuation::DirectValuationAnalysis)
{
    type Input = CompoundState<
        crate::analysis::cpa::lattice::pcode::PcodeAddressLattice,
        crate::analysis::direct_valuation::DirectValuationState
    >;

    fn make_initial_state(&self, addr: crate::modeling::machine::cpu::concrete::ConcretePcodeAddress) -> Self::Input {
        let location_state = self.0.make_initial_state(addr);
        let valuation_state = self.1.make_initial_state(addr);
        CompoundState::new(location_state, valuation_state)
    }

    fn make_output(&mut self, states: Vec<Self::State>) -> Vec<Self::State> {
        states
    }
}
