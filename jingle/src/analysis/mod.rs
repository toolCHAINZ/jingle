use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::LocationState;
use crate::analysis::cpa::{
    ConfigurableProgramAnalysis, IntoState, RunnableConfigurableProgramAnalysis,
};
use crate::analysis::pcode_store::{EntryPoint, PcodeStore};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

pub mod cfg;
pub mod compound;
pub mod cpa;
pub mod ctl;
pub mod location;
#[expect(unused)]
mod path;
pub mod pcode_store;
pub mod valuation;
pub mod varnode;
pub mod varnode_map;

/// A trait for analyses that can be run. This is automatically implemented for all
/// [`Analysis`] types whose CPA state implements [`LocationState`].
///
/// Types can override the default `run` implementation to provide custom behavior.
pub trait Analysis
where
    Self: RunnableConfigurableProgramAnalysis,
    Self::State: LocationState,
{
    /// Run the [Analysis] and return the reached states
    ///
    /// The default implementation uses the standard CPA algorithm and delegates
    /// to `make_output` for any post-processing. Types can override this to provide
    /// custom run behavior.
    fn run<T: PcodeStore, I: IntoState<Self>>(
        &self,
        store: T,
        initial_state: I,
    ) -> <Self::Reducer as Residue<Self::State>>::Output {
        // Use the CPA's `make_initial_state` helper so CPAs that need access to `self`
        // when constructing their initial state can do so. The default `make_initial_state`
        // simply calls `.into()` so this is fully backwards compatible.
        let initial = initial_state.into_state(self);

        self.run_cpa(initial, &store)
    }
}

/// Blanket implementation: any Analysis with LocationState automatically gets RunnableAnalysis
/// with the default run implementation
impl<T> Analysis for T
where
    T: RunnableConfigurableProgramAnalysis,
    T::State: LocationState,
{
}

pub trait AnalyzableEntry: PcodeStore + EntryPoint + Sized {
    fn run_analysis<T: Analysis>(&self, t: T) -> <T::Reducer as Residue<T::State>>::Output
    where
        <T as ConfigurableProgramAnalysis>::State: LocationState,
        ConcretePcodeAddress: IntoState<T>,
    {
        let state = self.get_entry();
        t.run(self, state)
    }
}

impl<T: PcodeStore + EntryPoint> AnalyzableEntry for T {}
