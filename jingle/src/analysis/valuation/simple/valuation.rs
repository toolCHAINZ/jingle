use std::collections::HashMap;

use internment::Intern;
use jingle_sleigh::VarNode;

use crate::analysis::{valuation::SimpleValue, varnode_map::VarNodeMap};

/// A container holding both direct writes (varnode -> value) and indirect writes
/// ([pointer expression] -> value) produced by stores.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SimpleValuation {
    pub direct_writes: VarNodeMap<SimpleValue>,
    /// Note: for now we are making the simplifying assumption
    /// that all indirect writes happen in one space; this hashmap
    /// can be keyed by both simpleValue and SpaceIndex to generalize this
    pub indirect_writes: HashMap<SimpleValue, SimpleValue>,
}

impl SimpleValuation {
    pub fn new() -> Self {
        Self {
            direct_writes: VarNodeMap::new(),
            indirect_writes: HashMap::new(),
        }
    }
}

pub enum SingleValuationLocation {
    Direct(Intern<VarNode>),
    Indirect(Intern<SimpleValue>),
}

pub struct SingleValuation {
    location: SingleValuationLocation,
    value: Intern<SimpleValue>,
}
