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

impl SingleValuation {
    /// Construct a `SingleValuation` representing a direct write.
    pub fn new_direct(vn: VarNode, value: SimpleValue) -> Self {
        Self {
            location: SingleValuationLocation::Direct(Intern::new(vn)),
            value: Intern::new(value),
        }
    }

    /// Construct a `SingleValuation` representing an indirect write (pointer expression).
    pub fn new_indirect(ptr: SimpleValue, value: SimpleValue) -> Self {
        Self {
            location: SingleValuationLocation::Indirect(Intern::new(ptr)),
            value: Intern::new(value),
        }
    }

    /// Access the location (direct/indirect) of this valuation.
    pub fn location(&self) -> &SingleValuationLocation {
        &self.location
    }

    /// Access the value for this valuation.
    pub fn value(&self) -> &SimpleValue {
        self.value.as_ref()
    }
}

/// Iterator over the contents of a `SimpleValuation`.
///
/// The iterator holds a reference to the originating `SimpleValuation` and
/// yields `SingleValuation` items for each direct and indirect write.
pub struct SimpleValuationIter<'a> {
    _valuation: &'a SimpleValuation,
    direct_entries: Vec<(Intern<VarNode>, Intern<SimpleValue>)>,
    direct_idx: usize,
    indirect_entries: Vec<(Intern<SimpleValue>, Intern<SimpleValue>)>,
    indirect_idx: usize,
}

impl<'a> SimpleValuationIter<'a> {
    fn new(valuation: &'a SimpleValuation) -> Self {
        // Collect direct entries (clone into interns so the iterator can be self-contained).
        let mut direct_entries: Vec<(Intern<VarNode>, Intern<SimpleValue>)> = Vec::new();
        for (vn, val) in valuation.direct_writes.items() {
            direct_entries.push((Intern::new(vn.clone()), Intern::new(val.clone())));
        }

        // Collect indirect entries (pointer expression -> value).
        let mut indirect_entries: Vec<(Intern<SimpleValue>, Intern<SimpleValue>)> = Vec::new();
        for (ptr, val) in &valuation.indirect_writes {
            indirect_entries.push((Intern::new(ptr.clone()), Intern::new(val.clone())));
        }

        Self {
            _valuation: valuation,
            direct_entries,
            direct_idx: 0,
            indirect_entries,
            indirect_idx: 0,
        }
    }
}

impl<'a> Iterator for SimpleValuationIter<'a> {
    type Item = SingleValuation;

    fn next(&mut self) -> Option<Self::Item> {
        // Yield all direct entries first, then indirect entries.
        if self.direct_idx < self.direct_entries.len() {
            let (vn_intern, val_intern) = self.direct_entries[self.direct_idx].clone();
            self.direct_idx += 1;
            return Some(SingleValuation {
                location: SingleValuationLocation::Direct(vn_intern),
                value: val_intern,
            });
        }

        if self.indirect_idx < self.indirect_entries.len() {
            let (ptr_intern, val_intern) = self.indirect_entries[self.indirect_idx].clone();
            self.indirect_idx += 1;
            return Some(SingleValuation {
                location: SingleValuationLocation::Indirect(ptr_intern),
                value: val_intern,
            });
        }

        None
    }
}

impl<'a> IntoIterator for &'a SimpleValuation {
    type Item = SingleValuation;
    type IntoIter = SimpleValuationIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SimpleValuationIter::new(self)
    }
}
