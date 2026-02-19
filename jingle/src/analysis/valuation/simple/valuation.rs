use std::borrow::Borrow;
use std::collections::BTreeMap;

use crate::display::JingleDisplay;
use internment::Intern;
use jingle_sleigh::{SleighArchInfo, VarNode};
use std::fmt::{Display, Formatter};

use crate::analysis::{valuation::SimpleValue, varnode_map::VarNodeMap};

/// A container holding both direct writes (varnode -> value) and indirect writes
/// ([pointer expression] -> value) produced by stores.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SimpleValuation {
    pub direct_writes: VarNodeMap<SimpleValue>,
    /// Note: for now we are making the simplifying assumption
    /// that all indirect writes happen in one space; this hashmap
    /// can be keyed by both simpleValue and SpaceIndex to generalize this
    pub indirect_writes: BTreeMap<SimpleValue, SimpleValue>,
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
            indirect_writes: BTreeMap::new(),
        }
    }

    /// Construct a `SimpleValuation` with the provided direct and indirect write maps.
    ///
    /// This allows callers to build a `SimpleValuation` with pre-populated contents
    /// instead of creating an empty one and inserting entries afterwards.
    pub fn with_contents(
        direct_writes: VarNodeMap<SimpleValue>,
        indirect_writes: BTreeMap<SimpleValue, SimpleValue>,
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

    /// Returns the number of entries (both direct and indirect) in this valuation.
    pub fn len(&self) -> usize {
        self.direct_writes.len() + self.indirect_writes.len()
    }

    /// Returns `true` if this valuation contains no entries.
    pub fn is_empty(&self) -> bool {
        self.direct_writes.is_empty() && self.indirect_writes.is_empty()
    }

    /// Returns an iterator over all locations (keys) in this valuation.
    pub fn keys(&self) -> Keys<'_> {
        Keys::new(self)
    }

    /// Alias for `keys()` to provide a more intuitive API for accessing valuation locations.
    pub fn locations(&self) -> Keys<'_> {
        self.keys()
    }

    /// Returns an iterator over all values in this valuation.
    pub fn values(&self) -> Values<'_> {
        Values::new(self)
    }

    /// Returns a mutable iterator over all values in this valuation.
    pub fn values_mut(&mut self) -> ValuesMut<'_> {
        ValuesMut::new(self)
    }

    pub fn iter(&self) -> SimpleValuationIter<'_> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> SimpleValuationIterMut<'_> {
        SimpleValuationIterMut::new(self)
    }

    pub fn remove_value_from(&mut self, loc: &SingleValuationLocation) {
        match loc {
            SingleValuationLocation::Direct(vn_intern) => {
                self.direct_writes.remove(vn_intern.as_ref());
            }
            SingleValuationLocation::Indirect(ptr_intern) => {
                self.indirect_writes.remove(ptr_intern.as_ref());
            }
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

// Allow converting a raw `VarNode` directly into a `SingleValuationLocation::Direct`.
impl From<&VarNode> for SingleValuationLocation {
    fn from(vn: &VarNode) -> Self {
        SingleValuationLocation::Direct(Intern::new(vn.clone()))
    }
}

// Allow converting a `SimpleValue` directly into a `SingleValuationLocation::Indirect`.
impl From<SimpleValue> for SingleValuationLocation {
    fn from(ptr: SimpleValue) -> Self {
        SingleValuationLocation::Indirect(Intern::new(ptr))
    }
}

// Allow converting a `SimpleValue` directly into a `SingleValuationLocation::Indirect`.
impl From<&SimpleValue> for SingleValuationLocation {
    fn from(ptr: &SimpleValue) -> Self {
        SingleValuationLocation::Indirect(Intern::new(ptr.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
/// Yields tuples of `(SingleValuationLocation, &SimpleValue)` for each entry,
/// matching the API of `iter_mut()` and following standard library conventions.
pub struct SimpleValuationIter<'a> {
    direct_iter: crate::analysis::varnode_map::Iter<'a, SimpleValue>,
    indirect_iter: std::collections::btree_map::Iter<'a, SimpleValue, SimpleValue>,
    direct_done: bool,
}

impl<'a> SimpleValuationIter<'a> {
    pub fn new(valuation: &'a SimpleValuation) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter(),
            indirect_iter: valuation.indirect_writes.iter(),
            direct_done: false,
        }
    }
}

impl<'a> Iterator for SimpleValuationIter<'a> {
    type Item = (SingleValuationLocation, &'a SimpleValue);

    fn next(&mut self) -> Option<Self::Item> {
        // First, iterate through all direct entries
        if !self.direct_done {
            if let Some((vn, val)) = self.direct_iter.next() {
                let location = SingleValuationLocation::Direct(Intern::new(vn.clone()));
                return Some((location, val));
            }
            self.direct_done = true;
        }

        // Then iterate through indirect entries
        if let Some((ptr, val)) = self.indirect_iter.next() {
            let location = SingleValuationLocation::Indirect(Intern::new(ptr.clone()));
            return Some((location, val));
        }

        None
    }
}

impl<'a> IntoIterator for &'a SimpleValuation {
    type Item = (SingleValuationLocation, &'a SimpleValue);
    type IntoIter = SimpleValuationIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SimpleValuationIter::new(self)
    }
}

/// A mutable iterator over the contents of a `SimpleValuation`.
///
/// Yields mutable references to both the location and value of each entry.
pub struct SimpleValuationIterMut<'a> {
    direct_iter: crate::analysis::varnode_map::IterMut<'a, SimpleValue>,
    indirect_iter: std::collections::btree_map::IterMut<'a, SimpleValue, SimpleValue>,
    direct_done: bool,
}

impl<'a> SimpleValuationIterMut<'a> {
    pub fn new(valuation: &'a mut SimpleValuation) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter_mut(),
            indirect_iter: valuation.indirect_writes.iter_mut(),
            direct_done: false,
        }
    }
}

impl<'a> Iterator for SimpleValuationIterMut<'a> {
    type Item = (SingleValuationLocation, &'a mut SimpleValue);

    fn next(&mut self) -> Option<Self::Item> {
        // First, iterate through all direct entries
        if !self.direct_done {
            if let Some((vn, val)) = self.direct_iter.next() {
                let location = SingleValuationLocation::Direct(Intern::new(vn.clone()));
                return Some((location, val));
            }
            self.direct_done = true;
        }

        // Then iterate through indirect entries
        if let Some((ptr, val)) = self.indirect_iter.next() {
            let location = SingleValuationLocation::Indirect(Intern::new(ptr.clone()));
            return Some((location, val));
        }

        None
    }
}

/// An iterator over the keys (locations) of a `SimpleValuation`.
///
/// This struct is created by the `keys` method on `SimpleValuation`.
pub struct Keys<'a> {
    direct_iter: crate::analysis::varnode_map::Iter<'a, SimpleValue>,
    indirect_iter: std::collections::btree_map::Iter<'a, SimpleValue, SimpleValue>,
    direct_done: bool,
}

impl<'a> Keys<'a> {
    pub fn new(valuation: &'a SimpleValuation) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter(),
            indirect_iter: valuation.indirect_writes.iter(),
            direct_done: false,
        }
    }
}

impl<'a> Iterator for Keys<'a> {
    type Item = SingleValuationLocation;

    fn next(&mut self) -> Option<Self::Item> {
        // First, iterate through all direct entries
        if !self.direct_done {
            if let Some((vn, _)) = self.direct_iter.next() {
                return Some(SingleValuationLocation::Direct(Intern::new(vn.clone())));
            }
            self.direct_done = true;
        }

        // Then iterate through indirect entries
        if let Some((ptr, _)) = self.indirect_iter.next() {
            return Some(SingleValuationLocation::Indirect(Intern::new(ptr.clone())));
        }

        None
    }
}

/// An iterator over the values of a `SimpleValuation`.
///
/// This struct is created by the `values` method on `SimpleValuation`.
pub struct Values<'a> {
    direct_iter: crate::analysis::varnode_map::Iter<'a, SimpleValue>,
    indirect_iter: std::collections::btree_map::Iter<'a, SimpleValue, SimpleValue>,
    direct_done: bool,
}

impl<'a> Values<'a> {
    pub fn new(valuation: &'a SimpleValuation) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter(),
            indirect_iter: valuation.indirect_writes.iter(),
            direct_done: false,
        }
    }
}

impl<'a> Iterator for Values<'a> {
    type Item = &'a SimpleValue;

    fn next(&mut self) -> Option<Self::Item> {
        // First, iterate through all direct entries
        if !self.direct_done {
            if let Some((_, val)) = self.direct_iter.next() {
                return Some(val);
            }
            self.direct_done = true;
        }

        // Then iterate through indirect entries
        if let Some((_, val)) = self.indirect_iter.next() {
            return Some(val);
        }

        None
    }
}

/// A mutable iterator over the values of a `SimpleValuation`.
///
/// This struct is created by the `values_mut` method on `SimpleValuation`.
pub struct ValuesMut<'a> {
    direct_iter: crate::analysis::varnode_map::IterMut<'a, SimpleValue>,
    indirect_iter: std::collections::btree_map::IterMut<'a, SimpleValue, SimpleValue>,
    direct_done: bool,
}

impl<'a> ValuesMut<'a> {
    pub fn new(valuation: &'a mut SimpleValuation) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter_mut(),
            indirect_iter: valuation.indirect_writes.iter_mut(),
            direct_done: false,
        }
    }
}

impl<'a> Iterator for ValuesMut<'a> {
    type Item = &'a mut SimpleValue;

    fn next(&mut self) -> Option<Self::Item> {
        // First, iterate through all direct entries
        if !self.direct_done {
            if let Some((_, val)) = self.direct_iter.next() {
                return Some(val);
            }
            self.direct_done = true;
        }

        // Then iterate through indirect entries
        if let Some((_, val)) = self.indirect_iter.next() {
            return Some(val);
        }

        None
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
    type Item = (SingleValuationLocation, &'a mut SimpleValue);
    type IntoIter = SimpleValuationIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SimpleValuationIterMut::new(self)
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

// todo: allow multiple valuations at the same location
// requires a refactor to all the internal datastructures, but is likely necessary to
// express multiple requirements
// alternatively, add an And node to SimpleValue and use that? THen we can structurally search for
// it...
impl FromIterator<SingleValuation> for SimpleValuation {
    fn from_iter<T: IntoIterator<Item = SingleValuation>>(iter: T) -> Self {
        let mut s = SimpleValuation::new();
        for sv in iter {
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

#[cfg(test)]
mod tests {
    use super::*;
    use jingle_sleigh::VarNode;

    #[test]
    fn test_iter_yields_tuples() {
        let mut valuation = SimpleValuation::new();
        let vn = VarNode {
            space_index: 0,
            offset: 0x1000,
            size: 8,
        };
        valuation
            .direct_writes
            .insert(vn.clone(), SimpleValue::const_(42));

        // iter() should yield (location, &value) tuples
        let mut count = 0;
        for (loc, val) in valuation.iter() {
            count += 1;
            assert!(matches!(loc, SingleValuationLocation::Direct(_)));
            assert_eq!(*val, SimpleValue::const_(42));
        }
        assert_eq!(count, 1);
    }

    #[test]
    fn test_iter_mut_yields_tuples() {
        let mut valuation = SimpleValuation::new();
        let vn = VarNode {
            space_index: 0,
            offset: 0x1000,
            size: 8,
        };
        valuation
            .direct_writes
            .insert(vn.clone(), SimpleValue::const_(42));

        // iter_mut() should yield (location, &mut value) tuples
        for (loc, val) in valuation.iter_mut() {
            assert!(matches!(loc, SingleValuationLocation::Direct(_)));
            *val = SimpleValue::const_(100);
        }

        // Verify mutation worked
        assert_eq!(
            valuation.direct_writes.get(&vn),
            Some(&SimpleValue::const_(100))
        );
    }

    #[test]
    fn test_into_iter_yields_entries() {
        let mut valuation = SimpleValuation::new();
        let vn = VarNode {
            space_index: 0,
            offset: 0x1000,
            size: 8,
        };
        valuation
            .direct_writes
            .insert(vn.clone(), SimpleValue::const_(42));

        // into_iter() should yield owned SingleValuation entries
        let mut count = 0;
        for entry in valuation {
            count += 1;
            assert!(matches!(entry.location, SingleValuationLocation::Direct(_)));
            assert_eq!(*entry.value.as_ref(), SimpleValue::const_(42));
        }
        assert_eq!(count, 1);
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut valuation = SimpleValuation::new();
        assert_eq!(valuation.len(), 0);
        assert!(valuation.is_empty());

        let vn = VarNode {
            space_index: 0,
            offset: 0x1000,
            size: 8,
        };
        valuation
            .direct_writes
            .insert(vn.clone(), SimpleValue::const_(42));

        assert_eq!(valuation.len(), 1);
        assert!(!valuation.is_empty());

        // Add an indirect write
        valuation
            .indirect_writes
            .insert(SimpleValue::const_(100), SimpleValue::const_(200));

        assert_eq!(valuation.len(), 2);
        assert!(!valuation.is_empty());
    }

    #[test]
    fn test_keys_iterator() {
        let mut valuation = SimpleValuation::new();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0x1000,
            size: 8,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 0x2000,
            size: 8,
        };

        valuation
            .direct_writes
            .insert(vn1.clone(), SimpleValue::const_(42));
        valuation
            .direct_writes
            .insert(vn2.clone(), SimpleValue::const_(99));

        let keys: Vec<_> = valuation.keys().collect();
        assert_eq!(keys.len(), 2);
        for key in keys {
            assert!(matches!(key, SingleValuationLocation::Direct(_)));
        }
    }

    #[test]
    fn test_values_iterator() {
        let mut valuation = SimpleValuation::new();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0x1000,
            size: 8,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 0x2000,
            size: 8,
        };

        valuation
            .direct_writes
            .insert(vn1.clone(), SimpleValue::const_(42));
        valuation
            .direct_writes
            .insert(vn2.clone(), SimpleValue::const_(99));

        let values: Vec<_> = valuation.values().collect();
        assert_eq!(values.len(), 2);
        assert!(values.contains(&&SimpleValue::const_(42)));
        assert!(values.contains(&&SimpleValue::const_(99)));
    }

    #[test]
    fn test_values_mut_iterator() {
        let mut valuation = SimpleValuation::new();
        let vn = VarNode {
            space_index: 0,
            offset: 0x1000,
            size: 8,
        };

        valuation
            .direct_writes
            .insert(vn.clone(), SimpleValue::const_(42));

        // Mutate all values
        for val in valuation.values_mut() {
            *val = SimpleValue::const_(1000);
        }

        // Verify mutation worked
        assert_eq!(
            valuation.direct_writes.get(&vn),
            Some(&SimpleValue::const_(1000))
        );
    }
}
