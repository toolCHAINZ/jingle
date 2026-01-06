use crate::analysis::cpa::{ConfigurableProgramAnalysis, RunnableConfigurableProgramAnalysis};
use crate::analysis::cpa::state::LocationState;
use crate::analysis::pcode_store::{EntryPoint, PcodeStore};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

pub mod back_edge;
// mod bounded_visit;
pub mod cfg;
pub mod cpa;
pub mod direct_location;
// mod location;
mod bmc;
pub mod bounded_visit;
pub mod ctl;
#[expect(unused)]
mod path;
pub mod pcode_store;
pub mod unwinding;
pub mod varnode;
pub mod stack_offset;
pub mod compound;

/// A compatibility wrapper around types implementing the Configurable Program Analysis (CPA).
/// The intent here is to provide some structure for running and combining CPAs. The output of the CPA
/// is often not exactly in a format that is easily used, and they can require some setup. This trait
/// allows for specifying a way to define the CPA's input (assuming a
/// [PcodeCfg](crate::analysis::cfg::PcodeCfg), indexed by
/// [ConcretePcodeAddress]s), and process its output. 
///
/// This trait can be implemented by types that may or may not be runnable. For runnable analyses,
/// see [`RunnableAnalysis`].
pub trait Analysis: ConfigurableProgramAnalysis {
    /// The output type of the analysis; may or may not be the CPA's result
    type Output;
    /// The input type of the analysis, must be derivable from a [ConcretePcodeAddress] and
    /// any state in the type implementing [Analysis]
    type Input: Into<Self::State>;

    /// Given an initial [ConcretePcodeAddress], derive the [Input](Self::Input) state for
    /// a CPA
    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input;

    /// Produce the output of the analysis
    fn make_output(&mut self, states: &[Self::State]) -> Self::Output;
}

/// A trait for analyses that can be run. This is automatically implemented for all
/// [`Analysis`] types whose CPA state implements [`LocationState`].
/// 
/// Types can override the default `run` implementation to provide custom behavior.
pub trait RunnableAnalysis: Analysis 
where 
    Self: RunnableConfigurableProgramAnalysis,
    Self::State: LocationState,
{
    /// Run the [Analysis] and return its [Output](Analysis::Output)
    /// 
    /// The default implementation uses the standard CPA algorithm and delegates
    /// output generation to `make_output`. Types can override this to provide
    /// custom run behavior.
    fn run<T: PcodeStore, I: Into<Self::Input>>(&mut self, store: T, initial_state: I) -> Self::Output {
        let initial_state = initial_state.into();
        let i = self.run_cpa(initial_state.into(), &store);
        self.make_output(&i)
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

pub trait AnalyzableBase: PcodeStore + Sized {
    fn run_analysis_at<T: RunnableAnalysis, S: Into<ConcretePcodeAddress>>(
        &self,
        entry: S,
        mut t: T,
    ) -> T::Output 
    where 
        <T as ConfigurableProgramAnalysis>::State: LocationState 
    {
        let addr = entry.into();
        let entry = t.make_initial_state(addr);
        t.run(self, entry)
    }
}

pub trait AnalyzableEntry: PcodeStore + EntryPoint + Sized {
    fn run_analysis<T: RunnableAnalysis>(&self, mut t: T) -> T::Output 
    where 
        <T as ConfigurableProgramAnalysis>::State: LocationState 
    {
        let entry = t.make_initial_state(self.get_entry());
        t.run(self, entry)
    }
}

impl<T: PcodeStore> AnalyzableBase for T {}
impl<T: PcodeStore + EntryPoint> AnalyzableEntry for T {}
