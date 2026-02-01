use crate::analysis::cpa::residue::CfgReducer;

use crate::analysis::cpa::ConfigurableProgramAnalysis;
use crate::analysis::location::basic::state::{BasicLocationState, CallBehavior};

pub mod state;

#[cfg(test)]
mod tests;

pub struct BasicLocationAnalysis {
    call_behavior: CallBehavior,
}

impl BasicLocationAnalysis {
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

impl ConfigurableProgramAnalysis for BasicLocationAnalysis {
    type State = BasicLocationState;
    type Reducer<'op> = CfgReducer<'op, Self::State>;
}
