use crate::analysis::cpa::residue::Residue;
use crate::analysis::cpa::state::LocationState;
use crate::analysis::cpa::{
    ConfigurableProgramAnalysis, IntoState, RunnableConfigurableProgramAnalysis,
};
use crate::analysis::pcode_store::{EntryPoint, PcodeStore};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

pub mod back_edge;
// mod bounded_visit;
mod bmc;
pub mod bounded_branch;
pub mod cfg;
pub mod compound;
pub mod cpa;
pub mod ctl;
pub mod direct_valuation;
pub mod direct_valuation2;
pub mod location;
#[expect(unused)]
mod path;
pub mod pcode_store;
pub mod varnode;
pub mod varnode_map;
/// A compatibility wrapper around types implementing the Configurable Program Analysis (CPA).
/// The intent here is to provide some structure for running and combining CPAs. This trait
/// allows for specifying a way to define the CPA's input (assuming a
/// [PcodeCfg](crate::analysis::cfg::PcodeCfg), indexed by
/// [ConcretePcodeAddress]s).
///
/// The output of an analysis is simply the `Vec<Self::State>` of reached states. Consumers
/// that need to extract additional information (e.g., built-up CFGs) should do so by accessing
/// the analysis struct directly after running.
///
/// This trait can be implemented by types that may or may not be runnable. For runnable analyses,
/// see [`RunnableAnalysis`].
pub trait Analysis: ConfigurableProgramAnalysis {}

/// A trait for analyses that can be run. This is automatically implemented for all
/// [`Analysis`] types whose CPA state implements [`LocationState`].
///
/// Types can override the default `run` implementation to provide custom behavior.
pub trait RunnableAnalysis: Analysis
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
        &mut self,
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
impl<T> RunnableAnalysis for T
where
    T: Analysis,
    T: RunnableConfigurableProgramAnalysis,
    T::State: LocationState,
{
}

pub trait AnalyzableEntry: PcodeStore + EntryPoint + Sized {
    fn run_analysis<T: RunnableAnalysis>(
        &self,
        mut t: T,
    ) -> <T::Reducer as Residue<T::State>>::Output
    where
        <T as ConfigurableProgramAnalysis>::State: LocationState,
        ConcretePcodeAddress: IntoState<T>,
    {
        let state = self.get_entry();
        t.run(self, state)
    }
}

impl<T: PcodeStore + EntryPoint> AnalyzableEntry for T {}
