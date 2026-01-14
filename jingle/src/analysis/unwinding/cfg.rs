#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum UnwoundLocation {
    UnwindError(ConcretePcodeAddress),
    Location(Vec<usize>, FlatLattice<ConcretePcodeAddress>),
}

impl PartialEq<ConcretePcodeAddress> for UnwoundLocation {
    fn eq(&self, other: &ConcretePcodeAddress) -> bool {
        match self {
            UnwindError(a) => a == other,
            Location(_, a) => a == other,
        }
    }
}

impl PartialEq<UnwoundLocation> for ConcretePcodeAddress {
    fn eq(&self, other: &UnwoundLocation) -> bool {
        match other {
            UnwindError(a) => a == self,
            Location(_, a) => a == self,
        }
    }
}

impl LowerHex for UnwoundLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let tag = match self {
            UnwoundLocation::UnwindError(_) => "_Stop".to_string(),
            UnwoundLocation::Location(a, _) => {
                let strs: Vec<_> = a.iter().map(|f| format!("{:x}", f)).collect();
                strs.join("_")
            }
        };
        write!(f, "{}", tag)
    }
}

#[derive(Debug, Clone, Eq)]
pub struct BackEdgeVisitCountState {
    back_edge_visits: HashMap<(ConcretePcodeAddress, ConcretePcodeAddress), usize>,
    max: usize,
}

impl BackEdgeVisitCountState {
    pub fn new(back_edges: BackEdges, max: usize) -> Self {
        BackEdgeVisitCountState {
            back_edge_visits: back_edges.iter().map(|k| (k, 0)).collect(),
            max,
        }
    }

    pub fn back_edge_str(&self) -> Vec<usize> {
        let mut sorted = self
            .back_edge_visits
            .clone()
            .into_iter()
            .collect::<Vec<(BackEdge, usize)>>();
        sorted.sort_by(|(a, _), (b, _)| match a.0.cmp(&b.0) {
            Ordering::Equal => a.1.cmp(&b.1),
            a => a,
        });
        let strs: Vec<_> = sorted.into_iter().map(|(_, size)| size).collect();
        strs
    }

    pub fn back_edge_count(&self, be: BackEdge) -> Option<usize> {
        self.back_edge_visits.get(&be).cloned()
    }
    pub fn increment_back_edge_count(&mut self, be: BackEdge) {
        if let Some(count) = self.back_edge_visits.get_mut(&be) {
            *count += 1;
        }
    }

    pub fn terminated(&self) -> bool {
        let back_edge_limit = self.back_edge_visits.values().any(|b| b >= &self.max);
        back_edge_limit
    }

    pub fn same_visit_counts(&self, other: &Self) -> bool {
        self.back_edge_visits.eq(&other.back_edge_visits)
    }

    pub fn max(&self) -> usize {
        self.max
    }
}

impl UnwoundLocation {
    pub fn location(&self) -> Option<ConcretePcodeAddress> {
        match self {
            UnwindError(a) => a.clone().into(),
            Location(_, a) => a.value().cloned(),
        }
    }

    pub fn is_unwind_error(&self) -> bool {
        matches!(self, UnwindError(_))
    }

    pub fn from_cpa_state(a: &UnwindingCpaState, _max: usize) -> Self {
        if a.terminated() {
            UnwindError(a.location())
        } else {
            Location(a.back_edge_str(), a.location())
        }
    }
}

impl CfgState for UnwoundLocation {
    type Model = MachineState;

    fn new_const(&self, i: &SleighArchInfo) -> Self::Model {
        MachineState::fresh_for_address(i, *self.location())
    }
    fn model_id(&self) -> String {
        format!("{:x}", self.location())
    }

    fn location(&self) -> Option<ConcretePcodeAddress> {
        Some(*self.location())
    }
}

pub type UnwoundPcodeCfg = ModeledPcodeCfg<UnwoundLocation, PcodeOperation>;

impl<D: ModelTransition<MachineState>> ModeledPcodeCfg<UnwoundLocation, D> {
    pub fn check_model(
        &self,
        location: &UnwoundLocation,
        ctl_model: CtlFormula<UnwoundLocation, D>,
    ) -> Bool {
        let mut visitor = PcodeCfgVisitor {
            location: location.clone(),
            cfg: self,
            visited_locations: Rc::new(RefCell::new(HashSet::new())),
        };
        ctl_model.check(&mut visitor)
    }
}
