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
    fn run<'op, T: PcodeStore<'op> + ?Sized, I: IntoState<Self>>(
        &self,
        store: &'op T,
        initial_state: I,
    ) -> <<Self as ConfigurableProgramAnalysis>::Reducer<'op> as Residue<'op, Self::State>>::Output
    where
        Self::State: 'op,
    {
        // Use the CPA's `make_initial_state` helper so CPAs that need access to `self`
        // when constructing their initial state can do so. The default `make_initial_state`
        // simply calls `.into()` so this is fully backwards compatible.
        let initial = initial_state.into_state(self);

        // Delegate to the generic `run_cpa` implementation, instantiating the reducer
        // for the `'op` lifetime and passing the pcode store by reference so callers
        // can provide borrowed `PcodeOpRef<'op>` values from the store without cloning.
        self.run_cpa(initial, store)
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

pub trait AnalyzableEntry: for<'op> PcodeStore<'op> + EntryPoint + Sized {
    fn run_analysis<'op, T: Analysis>(
        &'op self,
        t: T,
    ) -> <<T as ConfigurableProgramAnalysis>::Reducer<'op> as Residue<'op, T::State>>::Output
    where
        <T as ConfigurableProgramAnalysis>::State: LocationState + 'op,
        ConcretePcodeAddress: IntoState<T>,
    {
        let state = self.get_entry();
        // Pass `self` as the pcode store reference with lifetime `'op`.
        t.run(self, state)
    }
}

impl<T: for<'op> PcodeStore<'op> + EntryPoint> AnalyzableEntry for T {}
