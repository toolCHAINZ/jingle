use crate::analysis::Analysis;

use crate::analysis::cpa::reducer::CfgReducer;

use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::location::basic::state::{CallBehavior, DirectLocationState};

pub mod state;

#[cfg(test)]
mod tests;

pub struct DirectLocationAnalysis {
    call_behavior: CallBehavior,
}

impl DirectLocationAnalysis {
    pub fn call_behavior(&self) -> CallBehavior {
        self.call_behavior
    }

    pub fn set_call_behavior(&mut self, behavior: CallBehavior) {
        self.call_behavior = behavior;
    }

    pub fn new(call_behavior: CallBehavior) -> Self {
        Self { call_behavior }
    }
}

impl ConfigurableProgramAnalysis for DirectLocationAnalysis {
    type State = DirectLocationState;
    type Reducer = CfgReducer<Self::State>;
}

impl Analysis for DirectLocationAnalysis {}
