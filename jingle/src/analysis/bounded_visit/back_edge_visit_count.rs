use std::cmp::Ordering;
use crate::analysis::cpa::lattice::JoinSemiLattice;

#[derive(Debug, Eq, PartialEq, Ord, Clone)]
pub(crate) struct BackEdgeVisitCount<const N: usize>([usize; N]);

impl<const N: usize> Default for BackEdgeVisitCount<N>{
    fn default() -> Self {
        Self([0; N])
    }
}

impl<const N: usize> BackEdgeVisitCount<N> {
    pub(crate) fn increment(&mut self, p0: usize) {
        debug_assert!(p0 < N);
        self.0[p0] += 1;
    }
}

impl<const N: usize> PartialOrd for BackEdgeVisitCount<N> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut res = Ordering::Equal;
        for (ours, theirs) in self.0.iter().zip(other.0.iter()) {
            match res {
                Ordering::Equal => {
                    res = ours.cmp(theirs);
                }
                o => {
                    let curr = ours.cmp(theirs);
                    if o != curr {
                        if curr != Ordering::Equal {
                            return None;
                        }
                    }
                }
            }
        }
        Some(res)
    }
}

#[cfg(test)]
mod tests {
    use crate::analysis::bounded_visit::back_edge_visit_count::BackEdgeVisitCount;

    #[test]
    fn test_back_edge_visit_count() {
        let a = BackEdgeVisitCount([1, 2, 3]);
        let b = BackEdgeVisitCount([1, 2, 3]);
        let c = BackEdgeVisitCount([1, 2, 2]);
        let d = BackEdgeVisitCount([1, 2, 4]);
        let e = BackEdgeVisitCount([6, 2, 3]);
        assert_eq!(a, b);
        assert!(a > c);
        assert!(e > a);
        assert_eq!(d.partial_cmp(&e), None);
    }
}

impl<const N: usize> JoinSemiLattice for BackEdgeVisitCount<N>{
    fn join(&mut self, other: &Self) {
        for (a,b) in self.0.iter_mut().zip(other.0.iter()){
            *a = (*a).max(*b)
        }
    }
}