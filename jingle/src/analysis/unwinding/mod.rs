mod cfg;

use crate::analysis::back_edge::{BackEdgeCPA, BackEdges};
use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
use crate::analysis::cpa::residue::EmptyResidue;
use crate::analysis::cpa::state::LocationState;
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::pcode_store::PcodeStore;
use crate::analysis::{Analysis, RunnableAnalysis};
use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

pub use cfg::BackEdgeVisitCountState;

pub struct BoundedBackEdgeVisitAnalysis<A: ConfigurableProgramAnalysis> {
    inner: A,
    back_edges: BackEdges,
    max: usize,
}

impl<A: ConfigurableProgramAnalysis> BoundedBackEdgeVisitAnalysis<A>
where
    A::State: LocationState,
{
    /// Creates a new BoundedBackEdgeVisitAnalysis.
    ///
    /// This method automatically computes back-edges by running BackEdgeCPA internally.
    ///
    /// # Arguments
    /// * `inner` - The inner analysis to wrap with back-edge visit counting
    /// * `store` - The PcodeStore (e.g., LoadedSleighContext) to analyze
    /// * `start_addr` - The starting address for back-edge detection
    /// * `max` - Maximum number of times a back-edge can be visited
    pub fn new<T: PcodeStore>(
        inner: A,
        store: &T,
        start_addr: ConcretePcodeAddress,
        max: usize,
    ) -> Self {
        // Run BackEdgeCPA to compute back-edges
        let mut back_edge_analysis = BackEdgeCPA::new();
        let back_edges = back_edge_analysis.run(store, PcodeAddressLattice::Const(start_addr));

        Self {
            inner,
            back_edges,
            max,
        }
    }
}

impl<A: ConfigurableProgramAnalysis> ConfigurableProgramAnalysis for BoundedBackEdgeVisitAnalysis<A>
where
    A::State: LocationState,
{
    type State = BackEdgeVisitCountState<A::State>;
    type Reducer = EmptyResidue<Self::State>;
}

impl<A, I> IntoState<BoundedBackEdgeVisitAnalysis<A>> for I
where
    A: ConfigurableProgramAnalysis,
    A::State: LocationState,
    I: IntoState<A>,
{
    fn into_state(self, c: &BoundedBackEdgeVisitAnalysis<A>) -> BackEdgeVisitCountState<A::State> {
        let location = self.into_state(&c.inner);
        BackEdgeVisitCountState::new(location, c.back_edges.clone(), c.max)
    }
}

impl<A: ConfigurableProgramAnalysis> Analysis for BoundedBackEdgeVisitAnalysis<A> where
    A::State: LocationState
{
}
