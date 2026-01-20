mod cfg;

use crate::analysis::Analysis;
use crate::analysis::back_edge::BackEdges;
use crate::analysis::cpa::state::LocationState;
use crate::analysis::cpa::{ConfigurableProgramAnalysis, IntoState};
use crate::analysis::cpa::residue::EmptyResidue;

pub use cfg::BackEdgeVisitCountState;

pub struct BoundedBackEdgeVisitAnalysis<A: ConfigurableProgramAnalysis>
where
    A::State: LocationState,
{
    inner: A,
    back_edges: BackEdges,
    max: usize,
}

impl<A: ConfigurableProgramAnalysis> BoundedBackEdgeVisitAnalysis<A>
where
    A::State: LocationState,
{
    pub fn new(inner: A, back_edges: BackEdges, max: usize) -> Self {
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

impl<A: ConfigurableProgramAnalysis> Analysis for BoundedBackEdgeVisitAnalysis<A> where A::State: LocationState {}


