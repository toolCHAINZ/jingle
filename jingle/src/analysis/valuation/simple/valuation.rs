use std::borrow::Borrow;
use std::collections::HashMap;

use crate::display::JingleDisplay;
use internment::Intern;
use jingle_sleigh::{SleighArchInfo, VarNode};
use std::fmt::{Display, Formatter};

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

impl Default for SimpleValuation {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleValuation {
    pub fn new() -> Self {
        Self {
            direct_writes: VarNodeMap::new(),
            indirect_writes: HashMap::new(),
        }
    }

    /// Construct a `SimpleValuation` with the provided direct and indirect write maps.
    ///
    /// This allows callers to build a `SimpleValuation` with pre-populated contents
    /// instead of creating an empty one and inserting entries afterwards.
    pub fn with_contents(
        direct_writes: VarNodeMap<SimpleValue>,
        indirect_writes: HashMap<SimpleValue, SimpleValue>,
    ) -> Self {
        Self {
            direct_writes,
            indirect_writes,
        }
    }

    /// Lookup a value by a `SingleValuationLocation`.
    ///
    /// Accepts any type that can borrow a `SingleValuationLocation` (e.g. `&SingleValuationLocation`
    /// or `SingleValuationLocation`) and returns a reference to the stored `SimpleValue` if present.
    pub fn get<B: Borrow<SingleValuationLocation>>(&self, loc: B) -> Option<&SimpleValue> {
        match loc.borrow() {
            SingleValuationLocation::Direct(vn_intern) => {
                // VarNodeMap::get accepts anything that can borrow a VarNode
                self.direct_writes.get(vn_intern.as_ref())
            }
            SingleValuationLocation::Indirect(ptr_intern) => {
                // indirect_writes keyed by SimpleValue, lookup by reference to the SimpleValue
                self.indirect_writes.get(ptr_intern.as_ref())
            }
        }
    }

    pub fn iter(&self) -> SimpleValuationIter<'_> {
        self.into_iter()
    }
}

#[derive(Debug, Clone)]
pub enum SingleValuationLocation {
    Direct(Intern<VarNode>),
    Indirect(Intern<SimpleValue>),
}

impl SingleValuationLocation {
    /// Construct a `SingleValuationLocation` representing a direct location.
    pub fn new_direct(vn: VarNode) -> Self {
        SingleValuationLocation::Direct(Intern::new(vn))
    }

    /// Construct a `SingleValuationLocation` representing an indirect (pointer) location.
    pub fn new_indirect(ptr: SimpleValue) -> Self {
        SingleValuationLocation::Indirect(Intern::new(ptr))
    }
}

// Allow converting a raw `VarNode` directly into a `SingleValuationLocation::Direct`.
impl From<VarNode> for SingleValuationLocation {
    fn from(vn: VarNode) -> Self {
        SingleValuationLocation::Direct(Intern::new(vn))
    }
}

// Allow converting a `SimpleValue` directly into a `SingleValuationLocation::Indirect`.
impl From<SimpleValue> for SingleValuationLocation {
    fn from(ptr: SimpleValue) -> Self {
        SingleValuationLocation::Indirect(Intern::new(ptr))
    }
}

#[derive(Debug, Clone)]
pub struct SingleValuation {
    location: SingleValuationLocation,
    value: Intern<SimpleValue>,
}

impl SingleValuation {
    /// Construct a `SingleValuation` from a location and a value.
    /// The provided `value` will be interned.
    pub fn new(location: SingleValuationLocation, value: SimpleValue) -> Self {
        Self {
            location,
            value: Intern::new(value),
        }
    }
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

/// Add helper methods for mutating a `SimpleValuation`.
impl SimpleValuation {
    /// Add a single valuation into the appropriate map.
    ///
    /// Accepts any `loc` that can be converted into a `SingleValuationLocation` (e.g. a
    /// `VarNode` for direct locations or a `SimpleValue` for indirect locations) and any
    /// `value` that can be converted into a `SimpleValue`.
    ///
    /// Values are simplified before insertion to keep stored representations normalized.
    pub fn add<L, V>(&mut self, loc: L, value: V)
    where
        L: Into<SingleValuationLocation>,
        V: Into<SimpleValue>,
    {
        let loc = loc.into();
        let val = value.into().simplify();
        match loc {
            SingleValuationLocation::Direct(vn_intern) => {
                // VarNodeMap::insert expects an owned VarNode
                let vn = vn_intern.as_ref().clone();
                self.direct_writes.insert(vn, val);
            }
            SingleValuationLocation::Indirect(ptr_intern) => {
                // indirect_writes keyed by SimpleValue
                let ptr = ptr_intern.as_ref().clone();
                self.indirect_writes.insert(ptr, val);
            }
        }
    }
}

impl JingleDisplay for SingleValuationLocation {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        match self {
            SingleValuationLocation::Direct(vn_intern) => vn_intern.as_ref().fmt_jingle(f, info),
            SingleValuationLocation::Indirect(ptr_intern) => {
                // Display indirect locations as a bracketed pointer expression.
                write!(f, "[")?;
                ptr_intern.as_ref().fmt_jingle(f, info)?;
                write!(f, "]")
            }
        }
    }
}

impl Display for SingleValuationLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            // For direct locations we can rely on `VarNode`'s `Display` implementation.
            SingleValuationLocation::Direct(vn_intern) => write!(f, "{}", vn_intern.as_ref()),
            // `SimpleValue` does not implement `std::fmt::Display`, so fall back to `Debug`
            // (which is available) to provide a reasonable textual representation.
            SingleValuationLocation::Indirect(ptr_intern) => {
                write!(f, "[{}]", ptr_intern.as_ref())
            }
        }
    }
}

impl JingleDisplay for SingleValuation {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        // Reuse component displays for consistent formatting.
        write!(
            f,
            "{} = {}",
            self.location.display(info),
            self.value.as_ref().display(info)
        )
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
    pub fn new(valuation: &'a SimpleValuation) -> Self {
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
            let (vn_intern, val_intern) = self.direct_entries[self.direct_idx];
            self.direct_idx += 1;
            return Some(SingleValuation {
                location: SingleValuationLocation::Direct(vn_intern),
                value: val_intern,
            });
        }

        if self.indirect_idx < self.indirect_entries.len() {
            let (ptr_intern, val_intern) = self.indirect_entries[self.indirect_idx];
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

/// An owning iterator that consumes a `SimpleValuation` and yields `SingleValuation`
/// items without borrowing the original `SimpleValuation`.
pub struct SimpleValuationIntoIter {
    direct_entries: Vec<(Intern<VarNode>, Intern<SimpleValue>)>,
    direct_idx: usize,
    indirect_entries: Vec<(Intern<SimpleValue>, Intern<SimpleValue>)>,
    indirect_idx: usize,
}

impl Iterator for SimpleValuationIntoIter {
    type Item = SingleValuation;

    fn next(&mut self) -> Option<Self::Item> {
        if self.direct_idx < self.direct_entries.len() {
            let (vn_intern, val_intern) = self.direct_entries[self.direct_idx];
            self.direct_idx += 1;
            return Some(SingleValuation {
                location: SingleValuationLocation::Direct(vn_intern),
                value: val_intern,
            });
        }

        if self.indirect_idx < self.indirect_entries.len() {
            let (ptr_intern, val_intern) = self.indirect_entries[self.indirect_idx];
            self.indirect_idx += 1;
            return Some(SingleValuation {
                location: SingleValuationLocation::Indirect(ptr_intern),
                value: val_intern,
            });
        }

        None
    }
}

impl<'a> IntoIterator for &'a mut SimpleValuation {
    type Item = SingleValuation;
    type IntoIter = SimpleValuationIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SimpleValuationIter::new(self)
    }
}

impl IntoIterator for SimpleValuation {
    type Item = SingleValuation;
    type IntoIter = SimpleValuationIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        // Move direct entries, interning them as we go.
        let mut direct_entries: Vec<(Intern<VarNode>, Intern<SimpleValue>)> = Vec::new();
        for (vn, val) in self.direct_writes.into_iter() {
            direct_entries.push((Intern::new(vn), Intern::new(val)));
        }

        // Move indirect entries (pointer expression -> value).
        let mut indirect_entries: Vec<(Intern<SimpleValue>, Intern<SimpleValue>)> = Vec::new();
        for (ptr, val) in self.indirect_writes.into_iter() {
            indirect_entries.push((Intern::new(ptr), Intern::new(val)));
        }

        SimpleValuationIntoIter {
            direct_entries,
            direct_idx: 0,
            indirect_entries,
            indirect_idx: 0,
        }
    }
}

impl From<Vec<SingleValuation>> for SimpleValuation {
    fn from(vs: Vec<SingleValuation>) -> Self {
        let mut s = SimpleValuation::new();
        for sv in vs.into_iter() {
            // Obtain a cloned SimpleValue from the SingleValuation
            let val = sv.value();
            // Match on the location reference to insert into the appropriate map
            match sv.location() {
                SingleValuationLocation::Direct(vn_intern) => {
                    s.direct_writes
                        .insert(vn_intern.as_ref().clone(), val.clone());
                }
                SingleValuationLocation::Indirect(ptr_intern) => {
                    s.indirect_writes
                        .insert(ptr_intern.as_ref().clone(), val.clone());
                }
            }
        }
        s
    }
}

impl JingleDisplay for SimpleValuation {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        write!(f, "SimpleValuation {{")?;
        let mut first = true;

        // Direct writes (vn -> val)
        for (vn, val) in self.direct_writes.items() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{} = {}", vn.display(info), val.display(info))?;
        }

        // Indirect writes ([ptr_expr] -> val)
        for (ptr, val) in &self.indirect_writes {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "[{}] = {}", ptr.display(info), val.display(info))?;
        }

        write!(f, "}}")?;
        Ok(())
    }
}
