use crate::analysis::cfg::{CfgState, CfgStateModel, ModelTransition};

trait Observer: CfgStateModel {
    fn transition<T: ModelTransition<Self>>(&self, op: T) -> Option<Self>;
}