use crate::analysis::cfg::{CfgState, ModelTransition};

trait Observer: CfgState {
    fn transition<T: ModelTransition<Self>>(&self, op: T) -> Option<Self>;
}