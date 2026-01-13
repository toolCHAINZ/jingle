use crate::JingleError;
use crate::analysis::cpa::lattice::flat::FlatLattice;
use crate::modeling::machine::MachineState;
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;
use jingle_sleigh::{PcodeOperation, SleighArchInfo};
use std::fmt::Debug;
use std::hash::Hash;
use z3::ast::Bool;

/// An SMT Model of a state.
///
/// It has some notion of location, which is comparable to that of other states.
/// Separately, it has a notion of equality. Though not enforced the by the trait,
/// equality should always imply location equality. States can have PCodeOperations applied
/// to them, allowing for modeling state transitions
pub trait CfgStateModel: Debug + Clone + Sized {
    /// Returns a [`Bool`] indicating whether the location of this state is equal to that of
    /// another.
    fn location_eq(&self, other: &Self) -> Bool;

    /// Returns a [`Bool`] indicating whether this state is equal to another
    fn state_eq(&self, other: &Self) -> Bool;

    /// Derive a new state by applying a [`PcodeOperation`] to `self`.
    fn apply(&self, op: &PcodeOperation) -> Result<Self, JingleError>;
}

impl CfgStateModel for MachineState {
    fn location_eq(&self, other: &Self) -> Bool {
        self.pc().eq(other.pc())
    }

    fn state_eq(&self, other: &Self) -> Bool {
        let machine_eq = self.pc().machine.eq(&other.pc().machine);
        self.memory()._eq(other.memory(), &machine_eq)
    }

    fn apply(&self, op: &PcodeOperation) -> Result<Self, JingleError> {
        self.apply(op)
    }
}

/// A trait for types that support generating SMT models for pcode states. These states (and models)
/// may also include metadata outside the pcode state, such as unwinding counts and observer
/// automata states.
pub trait CfgState: Clone + Debug + Hash + Eq {
    /// A type representing a model of a [`CfgState`]
    type Model: CfgStateModel;

    /// Produces a model
    fn new_const(&self, i: &SleighArchInfo) -> Self::Model;

    /// Prefix used when producing SMT models of this state with `fresh`
    fn model_id(&self) -> String;

    /// Each CFG state is optionally associated with a concrete p-code address
    fn location(&self) -> Option<ConcretePcodeAddress>;
}

/// A trait representing the transition of states by a [`PcodeOperation`] or a sequence of
/// [`PcodeOperation`]s.
///
/// This reprresents transitions between the beginning and end of a node in a pcode CFG
pub trait ModelTransition<S: CfgStateModel>: Clone + Debug {
    fn transition(&self, init: &S) -> Result<S, JingleError>;
}

impl CfgState for ConcretePcodeAddress {
    type Model = MachineState;

    fn new_const(&self, i: &SleighArchInfo) -> Self::Model {
        MachineState::fresh_for_address(i, *self)
    }

    fn model_id(&self) -> String {
        format!("State_PC_{:x}_{:x}", self.machine, self.pcode)
    }

    fn location(&self) -> Option<ConcretePcodeAddress> {
        Some(*self)
    }
}

impl CfgState for FlatLattice<ConcretePcodeAddress> {
    type Model = MachineState;

    fn new_const(&self, i: &SleighArchInfo) -> Self::Model {
        match self {
            FlatLattice::Value(addr) => MachineState::fresh_for_address(i, addr),
            FlatLattice::Top => MachineState::fresh(i),
        }
    }

    fn model_id(&self) -> String {
        match self {
            FlatLattice::Value(a) => a.model_id(),
            FlatLattice::Top => "State_Top_".to_string(),
        }
    }

    fn location(&self) -> Option<ConcretePcodeAddress> {
        Option::from(*self)
    }
}

impl<N: CfgStateModel> ModelTransition<N> for PcodeOperation {
    fn transition(&self, init: &N) -> Result<N, JingleError> {
        init.apply(self)
    }
}

impl<N: CfgStateModel, T: ModelTransition<N>> ModelTransition<N> for Vec<T> {
    fn transition(&self, init: &N) -> Result<N, JingleError> {
        let mut state = init.clone();
        for op in self {
            state = op.transition(&state)?;
        }
        Ok(state)
    }
}

/// Implement `CfgState` for a couple of tuple arities where the first element
/// implements `CfgState`. These implementations delegate model creation and
/// location-related methods to the first (0th) element of the tuple.
///
/// We provide implementations for 2-, 3- and 4-tuples to cover common cases.
/// Variadic tuples are not supported in stable Rust, so add more arities if
/// needed.
impl<A: CfgState, B: Clone + Debug + Hash + Eq> CfgState for (A, B) {
    type Model = A::Model;

    fn new_const(&self, i: &SleighArchInfo) -> Self::Model {
        self.0.new_const(i)
    }

    fn model_id(&self) -> String {
        self.0.model_id()
    }

    fn location(&self) -> Option<ConcretePcodeAddress> {
        self.0.location()
    }
}

impl<A: CfgState, B: Clone + Debug + Hash + Eq, C: Clone + Debug + Hash + Eq> CfgState
    for (A, B, C)
{
    type Model = A::Model;

    fn new_const(&self, i: &SleighArchInfo) -> Self::Model {
        self.0.new_const(i)
    }

    fn model_id(&self) -> String {
        self.0.model_id()
    }

    fn location(&self) -> Option<ConcretePcodeAddress> {
        self.0.location()
    }
}

impl<
    A: CfgState,
    B: Clone + Debug + Hash + Eq,
    C: Clone + Debug + Hash + Eq,
    D: Clone + Debug + Hash + Eq,
> CfgState for (A, B, C, D)
{
    type Model = A::Model;

    fn new_const(&self, i: &SleighArchInfo) -> Self::Model {
        self.0.new_const(i)
    }

    fn model_id(&self) -> String {
        self.0.model_id()
    }

    fn location(&self) -> Option<ConcretePcodeAddress> {
        self.0.location()
    }
}
