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
/// [ConcretePcodeAddress]s), and process its output. A
/// Pcode CFG can then run any type implementing `Analysis` without a lot of wrangling. Analyses can
/// also output new PcodeCfgs or related types with additional information.
pub trait Analysis: RunnableConfigurableProgramAnalysis where Self::State : LocationState {
    /// The output type of the analysis; may or may not be the CPA's result
    type Output;
    /// The input type of the analysis, must be derivable from a [ConcretePcodeAddress] and
    /// any state in the type implementing [Analysis]
    type Input: Into<Self::State>;

    /// Given an initial [ConcretePcodeAddress], derive the [Input](Self::Input) state for
    /// a CPA
    fn make_initial_state(&self, addr: ConcretePcodeAddress) -> Self::Input;

    /// Produce the output of hte analysis
    fn make_output(&mut self, states: &[Self::State] ) -> Self::Output;

    /// Run the [Analysis] and return its [Output](Self::Output)
    fn run<T: PcodeStore, I: Into<Self::Input>>(&mut self, store: T, initial_state: I) -> Self::Output {
        let initial_state = initial_state.into();
        let i = self.run_cpa(initial_state.into(), &store);
        self.make_output(&i)
    }
}

pub trait AnalyzableBase: PcodeStore + Sized {
    fn run_analysis_at<T: Analysis, S: Into<ConcretePcodeAddress>>(
        &self,
        entry: S,
        mut t: T,
    ) -> T::Output where <T as ConfigurableProgramAnalysis>::State: LocationState {
        let addr = entry.into();
        let entry = t.make_initial_state(addr);
        t.run(self, entry)
    }
}

pub trait AnalyzableEntry: PcodeStore + EntryPoint + Sized {
    fn run_analysis<T: Analysis>(&self, mut t: T) -> T::Output where <T as ConfigurableProgramAnalysis>::State: LocationState {
        let entry = t.make_initial_state(self.get_entry());
        t.run(self, entry)
    }
}

impl<T: PcodeStore> AnalyzableBase for T {}
impl<T: PcodeStore + EntryPoint> AnalyzableEntry for T {}
