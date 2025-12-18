use std::{cell::RefCell, collections::HashSet, rc::Rc};

use jingle_sleigh::PcodeOperation;

use crate::{
    analysis::cfg::{CfgState, ModelTransition, PcodeCfg},
    modeling::machine::cpu::concrete::ConcretePcodeAddress,
};

#[derive(Clone)]
pub struct PcodeCfgVisitor<'a, N: CfgState = ConcretePcodeAddress, D = PcodeOperation> {
    pub(crate) cfg: &'a PcodeCfg<N, D>,
    pub(crate) location: N,
}

impl<'a, N: CfgState, D: ModelTransition<N::Model>> PcodeCfgVisitor<'a, N, D> {
    pub(crate) fn successors(&self) -> impl Iterator<Item = Self> {
        self.cfg
            .successors(&self.location)
            .into_iter()
            .flatten()
            .map(|n| Self {
                cfg: self.cfg,
                location: n.clone(),
            })
    }

    pub(crate) fn transition(&self) -> Option<&D> {
        self.cfg.ops.get(&self.location)
    }

    pub fn location(&self) -> &N {
        &self.location
    }

    pub fn state(&self) -> Option<&N::Model> {
        self.cfg.models.get(&self.location)
    }
}
