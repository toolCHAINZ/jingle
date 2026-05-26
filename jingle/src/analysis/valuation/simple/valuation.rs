use std::borrow::Borrow;
use std::collections::BTreeMap;

use crate::display::JingleDisplay;
use jingle_sleigh::{SleighArchInfo, VarNode};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::rc::Rc;

use crate::analysis::valuation::simple::value::IntoRcValue;
use crate::analysis::{valuation::Value, varnode_map::VarNodeMap};

mod btreemap_as_vec {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<K, V, S>(map: &BTreeMap<K, V>, s: S) -> Result<S::Ok, S::Error>
    where
        K: Serialize + Ord,
        V: Serialize,
        S: Serializer,
    {
        map.iter().collect::<Vec<_>>().serialize(s)
    }

    pub fn deserialize<'de, K, V, D>(d: D) -> Result<BTreeMap<K, V>, D::Error>
    where
        K: Deserialize<'de> + Ord,
        V: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Ok(Vec::<(K, V)>::deserialize(d)?.into_iter().collect())
    }
}

/// A container holding both direct writes (varnode -> value) and indirect writes
/// ([pointer expression] -> value) produced by stores.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ValuationSet {
    pub direct_writes: VarNodeMap<Rc<Value>>,
    /// Keyed on the load expression representing the memory location (e.g. `Load(ptr, size)`),
    /// not the raw pointer. This matches the `Value::Load` representation used when the
    /// stored value is read back by a load operation.
    /// Note: for now we are making the simplifying assumption that all indirect writes happen
    /// in one space; this map can be keyed by both `Value` and `SpaceIndex` to generalize.
    // todo: this should be more structured and probably just explicitly hold Loads
    //  anything downstream needing to express something more general should just use its
    //  own type instead of making the function of this type ambiguous
    #[serde(with = "btreemap_as_vec")]
    pub indirect_writes: BTreeMap<Value, Rc<Value>>,
}

impl Default for ValuationSet {
    fn default() -> Self {
        Self::new()
    }
}

impl ValuationSet {
    pub fn new() -> Self {
        Self {
            direct_writes: VarNodeMap::new(),
            indirect_writes: BTreeMap::new(),
        }
    }

    /// Construct a `ValuationSet` with the provided direct and indirect write maps.
    ///
    /// This allows callers to build a `ValuationSet` with pre-populated contents
    /// instead of creating an empty one and inserting entries afterwards.
    pub fn with_contents(
        direct_writes: VarNodeMap<Rc<Value>>,
        indirect_writes: BTreeMap<Value, Rc<Value>>,
    ) -> Self {
        Self {
            direct_writes,
            indirect_writes,
        }
    }

    /// Lookup a value by a `Location`.
    ///
    /// Accepts any type that can borrow a `Location` (e.g. `&Location`
    /// or `Location`) and returns a reference to the stored `Value` if present.
    pub fn get<B: Borrow<Location>>(&self, loc: B) -> Option<&Value> {
        match loc.borrow() {
            Location::Direct(vn) => self.direct_writes.get(vn).map(|rc| rc.as_ref()),
            Location::Indirect(ptr_intern) => {
                self.indirect_writes.get(ptr_intern).map(|rc| rc.as_ref())
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

    pub fn iter(&self) -> ValuationIter<'_> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> ValuationIterMut<'_> {
        ValuationIterMut::new(self)
    }

    pub fn remove_value_from(&mut self, loc: &Location) {
        match loc {
            Location::Direct(vn) => {
                self.direct_writes.remove(vn);
            }
            Location::Indirect(ptr_intern) => {
                self.indirect_writes.remove(ptr_intern);
            }
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Location {
    Direct(VarNode),
    Indirect(Value),
}

impl Location {
    /// Construct a `Location` representing a direct location.
    pub fn new_direct(vn: VarNode) -> Self {
        Location::Direct(vn)
    }

    /// Construct a `Location` representing an indirect (memory) location.
    /// `loc` must be a `Value::Load(...)` expression describing the actual location.
    pub fn new_indirect(loc: Value) -> Self {
        Location::Indirect(loc)
    }

    pub fn direct_covers(&self, other: &Self) -> bool {
        if let Location::Direct(vn1) = self
            && let Location::Direct(vn2) = other
        {
            vn1.covers(vn2)
        } else {
            false
        }
    }

    pub fn indirect(&self) -> Option<&Value> {
        match self {
            Self::Indirect(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_direct(&self) -> bool {
        matches!(self, Self::Direct(_))
    }

    pub fn is_indirect(&self) -> bool {
        matches!(self, Self::Indirect(_))
    }

    pub fn size(&self) -> Option<usize> {
        match self {
            Self::Direct(vn) => Some(vn.size()),
            Self::Indirect(Value::Load(load)) => Some(load.1),
            _ => None,
        }
    }

    pub fn as_value(&self) -> Value {
        match self {
            Self::Indirect(v) => v.clone(),
            Self::Direct(v) => Value::entry(*v),
        }
    }
}

// Allow converting a raw `VarNode` directly into a `Location::Direct`.
impl From<VarNode> for Location {
    fn from(vn: VarNode) -> Self {
        Location::Direct(vn)
    }
}

// Allow converting a raw `VarNode` directly into a `Location::Direct`.
impl From<&VarNode> for Location {
    fn from(vn: &VarNode) -> Self {
        Location::Direct(*vn)
    }
}

// Allow converting a `Value` directly into a `Location::Indirect`.
impl From<Value> for Location {
    fn from(ptr: Value) -> Self {
        Location::Indirect(ptr)
    }
}

// Allow converting a `Value` directly into a `Location::Indirect`.
impl From<&Value> for Location {
    fn from(ptr: &Value) -> Self {
        Location::Indirect(ptr.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Valuation {
    location: Location,
    value: Value,
}

impl Valuation {
    /// Construct a `Valuation` from a location and a value.
    /// The provided `value` will be interned.
    pub fn new(location: Location, value: Value) -> Self {
        Self { location, value }
    }
}

impl Valuation {
    /// Construct a `Valuation` representing a direct write.
    pub fn new_direct(vn: VarNode, value: Value) -> Self {
        Self {
            location: Location::Direct(vn),
            value,
        }
    }

    /// Construct a `Valuation` representing an indirect (memory) write.
    /// `loc` must be a `Value::Load(...)` expression describing the actual location.
    pub fn new_indirect(loc: Value, value: Value) -> Self {
        Self {
            location: Location::Indirect(loc),
            value,
        }
    }

    /// Access the location (direct/indirect) of this valuation.
    pub fn location(&self) -> &Location {
        &self.location
    }

    /// Access the value for this valuation.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Consume this valuation, returning its (location, value) parts by move.
    pub fn into_parts(self) -> (Location, Value) {
        (self.location, self.value)
    }
}

/// Add helper methods for mutating a `ValuationSet`.
impl ValuationSet {
    /// Add a single valuation into the appropriate map.
    ///
    /// Accepts any `loc` that can be converted into a `Location` (e.g. a
    /// `VarNode` for direct locations or a `Value` for indirect locations) and any
    /// `value` that can be converted into a `Value`.
    ///
    /// This is the single point where values are simplified before insertion.
    /// Callers must **not** pre-simplify values passed to this method; doing so
    /// causes redundant intern-arena acquisitions with no benefit.
    pub fn add<L, V>(&mut self, loc: L, value: V)
    where
        L: Into<Location>,
        V: IntoRcValue,
    {
        let loc = loc.into();
        let val = Value::simplify_shared(&value.into_rc());
        match loc {
            Location::Direct(vn) => {
                // Remove any existing entries whose range is entirely covered by this write.
                // Writing to a larger region (e.g. register[4:8]) physically overwrites all
                // sub-regions (e.g. register[4:4]) that fall within it.
                self.direct_writes
                    .retain(|existing, _| !vn.covers(existing) || existing == &vn);

                // When writing a sub-register, update any parent registers that cover it so
                // they reflect the partial write. E.g., writing AL must splice the new byte
                // into the stored RAX value. Collect first to avoid borrow conflicts.
                let parents: Vec<(VarNode, Rc<Value>)> = self
                    .direct_writes
                    .items()
                    .filter(|(existing, _)| existing.covers(&vn) && *existing != &vn)
                    .map(|(parent_vn, parent_val)| {
                        let byte_offset = (vn.offset() - parent_vn.offset()) as usize;
                        let merged =
                            Value::insert_bytes(parent_val, Rc::clone(&val), byte_offset);
                        (*parent_vn, Value::simplify_shared(&Rc::new(merged)))
                    })
                    .collect();

                for (parent_vn, merged_val) in parents {
                    self.direct_writes.insert(parent_vn, merged_val);
                }

                self.direct_writes.insert(vn, val);
            }
            Location::Indirect(ptr_intern) => {
                self.indirect_writes.insert(ptr_intern, val);
            }
        }
    }
}

impl JingleDisplay for Location {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        match self {
            Location::Direct(vn) => vn.fmt_jingle(f, info),
            Location::Indirect(loc_expr) => loc_expr.fmt_jingle(f, info),
        }
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Location::Direct(vn) => write!(f, "{}", vn),
            Location::Indirect(loc_expr) => write!(f, "{}", loc_expr),
        }
    }
}

impl JingleDisplay for Valuation {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        // Reuse component displays for consistent formatting.
        write!(
            f,
            "{} = {}",
            self.location.display(info),
            self.value.display(info)
        )
    }
}

/// Iterator over the contents of a `ValuationSet`.
///
/// Yields tuples of `(Location, &Value)` for each entry,
/// matching the API of `iter_mut()` and following standard library conventions.
pub struct ValuationIter<'a> {
    direct_iter: crate::analysis::varnode_map::Iter<'a, Rc<Value>>,
    indirect_iter: std::collections::btree_map::Iter<'a, Value, Rc<Value>>,
    direct_done: bool,
}

impl<'a> ValuationIter<'a> {
    pub fn new(valuation: &'a ValuationSet) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter(),
            indirect_iter: valuation.indirect_writes.iter(),
            direct_done: false,
        }
    }
}

impl<'a> Iterator for ValuationIter<'a> {
    type Item = (Location, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        // First, iterate through all direct entries
        if !self.direct_done {
            if let Some((vn, val)) = self.direct_iter.next() {
                return Some((Location::Direct(*vn), val.as_ref()));
            }
            self.direct_done = true;
        }

        // Then iterate through indirect entries
        if let Some((ptr, val)) = self.indirect_iter.next() {
            let location = Location::Indirect(ptr.clone());
            return Some((location, val.as_ref()));
        }

        None
    }
}

impl<'a> IntoIterator for &'a ValuationSet {
    type Item = (Location, &'a Value);
    type IntoIter = ValuationIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ValuationIter::new(self)
    }
}

/// A mutable iterator over the contents of a `ValuationSet`.
///
/// Yields mutable references to both the location and value of each entry.
pub struct ValuationIterMut<'a> {
    direct_iter: crate::analysis::varnode_map::IterMut<'a, Rc<Value>>,
    indirect_iter: std::collections::btree_map::IterMut<'a, Value, Rc<Value>>,
    direct_done: bool,
}

impl<'a> ValuationIterMut<'a> {
    pub fn new(valuation: &'a mut ValuationSet) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter_mut(),
            indirect_iter: valuation.indirect_writes.iter_mut(),
            direct_done: false,
        }
    }
}

impl<'a> Iterator for ValuationIterMut<'a> {
    type Item = (Location, &'a mut Rc<Value>);

    fn next(&mut self) -> Option<Self::Item> {
        // First, iterate through all direct entries
        if !self.direct_done {
            if let Some((vn, val)) = self.direct_iter.next() {
                return Some((Location::Direct(*vn), val));
            }
            self.direct_done = true;
        }

        // Then iterate through indirect entries
        if let Some((ptr, val)) = self.indirect_iter.next() {
            let location = Location::Indirect(ptr.clone());
            return Some((location, val));
        }

        None
    }
}

/// An iterator over the keys (locations) of a `ValuationSet`.
///
/// This struct is created by the `keys` method on `ValuationSet`.
pub struct Keys<'a> {
    direct_iter: crate::analysis::varnode_map::Iter<'a, Rc<Value>>,
    indirect_iter: std::collections::btree_map::Iter<'a, Value, Rc<Value>>,
    direct_done: bool,
}

impl<'a> Keys<'a> {
    pub fn new(valuation: &'a ValuationSet) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter(),
            indirect_iter: valuation.indirect_writes.iter(),
            direct_done: false,
        }
    }
}

impl<'a> Iterator for Keys<'a> {
    type Item = Location;

    fn next(&mut self) -> Option<Self::Item> {
        // First, iterate through all direct entries
        if !self.direct_done {
            if let Some((vn, _)) = self.direct_iter.next() {
                return Some(Location::Direct(*vn));
            }
            self.direct_done = true;
        }

        // Then iterate through indirect entries
        if let Some((ptr, _)) = self.indirect_iter.next() {
            return Some(Location::Indirect(ptr.clone()));
        }

        None
    }
}

/// An iterator over the values of a `ValuationSet`.
///
/// This struct is created by the `values` method on `ValuationSet`.
pub struct Values<'a> {
    direct_iter: crate::analysis::varnode_map::Iter<'a, Rc<Value>>,
    indirect_iter: std::collections::btree_map::Iter<'a, Value, Rc<Value>>,
    direct_done: bool,
}

impl<'a> Values<'a> {
    pub fn new(valuation: &'a ValuationSet) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter(),
            indirect_iter: valuation.indirect_writes.iter(),
            direct_done: false,
        }
    }
}

impl<'a> Iterator for Values<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        // First, iterate through all direct entries
        if !self.direct_done {
            if let Some((_, val)) = self.direct_iter.next() {
                return Some(val.as_ref());
            }
            self.direct_done = true;
        }

        // Then iterate through indirect entries
        if let Some((_, val)) = self.indirect_iter.next() {
            return Some(val.as_ref());
        }

        None
    }
}

/// A mutable iterator over the values of a `ValuationSet`.
///
/// This struct is created by the `values_mut` method on `ValuationSet`.
pub struct ValuesMut<'a> {
    direct_iter: crate::analysis::varnode_map::IterMut<'a, Rc<Value>>,
    indirect_iter: std::collections::btree_map::IterMut<'a, Value, Rc<Value>>,
    direct_done: bool,
}

impl<'a> ValuesMut<'a> {
    pub fn new(valuation: &'a mut ValuationSet) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter_mut(),
            indirect_iter: valuation.indirect_writes.iter_mut(),
            direct_done: false,
        }
    }
}

impl<'a> Iterator for ValuesMut<'a> {
    type Item = &'a mut Rc<Value>;

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

/// An owning iterator that consumes a `ValuationSet` and yields `Valuation`
/// items without borrowing the original `ValuationSet`.
pub struct ValuationIntoIter {
    direct_entries: std::vec::IntoIter<(VarNode, Rc<Value>)>,
    indirect_entries: std::vec::IntoIter<(Value, Rc<Value>)>,
    direct_done: bool,
}

impl Iterator for ValuationIntoIter {
    type Item = Valuation;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.direct_done {
            if let Some((vn, val)) = self.direct_entries.next() {
                let owned = Rc::try_unwrap(val).unwrap_or_else(|rc| (*rc).clone());
                return Some(Valuation::new_direct(vn, owned));
            }
            self.direct_done = true;
        }
        self.indirect_entries.next().map(|(ptr, val)| {
            let owned = Rc::try_unwrap(val).unwrap_or_else(|rc| (*rc).clone());
            Valuation::new_indirect(ptr, owned)
        })
    }
}

impl<'a> IntoIterator for &'a mut ValuationSet {
    type Item = (Location, &'a mut Rc<Value>);
    type IntoIter = ValuationIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ValuationIterMut::new(self)
    }
}

impl IntoIterator for ValuationSet {
    type Item = Valuation;
    type IntoIter = ValuationIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        ValuationIntoIter {
            direct_entries: self
                .direct_writes
                .into_iter()
                .collect::<Vec<_>>()
                .into_iter(),
            indirect_entries: self
                .indirect_writes
                .into_iter()
                .collect::<Vec<_>>()
                .into_iter(),
            direct_done: false,
        }
    }
}

impl From<Vec<Valuation>> for ValuationSet {
    fn from(vs: Vec<Valuation>) -> Self {
        let mut s = ValuationSet::new();
        for sv in vs {
            let (loc, val) = sv.into_parts();
            s.add(loc, val);
        }
        s
    }
}

// todo: allow multiple valuations at the same location
// requires a refactor to all the internal datastructures, but is likely necessary to
// express multiple requirements
// alternatively, add an And node to Value and use that? THen we can structurally search for
// it...
impl FromIterator<Valuation> for ValuationSet {
    fn from_iter<T: IntoIterator<Item = Valuation>>(iter: T) -> Self {
        let mut s = ValuationSet::new();
        for sv in iter {
            let (loc, val) = sv.into_parts();
            s.add(loc, val);
        }
        s
    }
}

impl Display for ValuationSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Valuation {{")?;
        let mut first = true;

        // Direct writes (vn -> val)
        for (vn, val) in self.direct_writes.items() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{} = {}", vn, val.as_ref())?;
        }

        // Indirect writes ([ptr_expr] -> val)
        for (ptr, val) in &self.indirect_writes {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "[{}] = {}", ptr, val.as_ref())?;
        }

        write!(f, "}}")?;
        Ok(())
    }
}

impl JingleDisplay for ValuationSet {
    fn fmt_jingle(&self, f: &mut Formatter<'_>, info: &SleighArchInfo) -> std::fmt::Result {
        write!(f, "Valuation {{")?;
        let mut first = true;

        // Direct writes (vn -> val)
        for (vn, val) in self.direct_writes.items() {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "{} = {}", vn.display(info), val.as_ref().display(info))?;
        }

        // Indirect writes ([ptr_expr] -> val)
        for (ptr, val) in &self.indirect_writes {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "[{}] = {}", ptr.display(info), val.as_ref().display(info))?;
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
        let mut valuation = ValuationSet::new();
        let vn = VarNode::new(0x1000, 8u32, 0u32);
        valuation.direct_writes.insert(vn, Rc::new(Value::const_(42, 8)));

        // iter() should yield (location, &value) tuples
        let mut count = 0;
        for (loc, val) in valuation.iter() {
            count += 1;
            assert!(matches!(loc, Location::Direct(_)));
            assert_eq!(*val, Value::const_(42, 8));
        }
        assert_eq!(count, 1);
    }

    #[test]
    fn test_iter_mut_yields_tuples() {
        let mut valuation = ValuationSet::new();
        let vn = VarNode::new(0x1000, 8u32, 0u32);
        valuation.direct_writes.insert(vn, Rc::new(Value::const_(42, 8)));

        // iter_mut() should yield (location, &mut Rc<Value>) tuples
        for (loc, val) in valuation.iter_mut() {
            assert!(matches!(loc, Location::Direct(_)));
            *val = Rc::new(Value::const_(100, 8));
        }

        // Verify mutation worked
        assert_eq!(
            valuation.get(Location::Direct(vn)),
            Some(&Value::const_(100, 8))
        );
    }

    #[test]
    fn test_into_iter_yields_entries() {
        let mut valuation = ValuationSet::new();
        let vn = VarNode::new(0x1000, 8u32, 0u32);
        valuation.direct_writes.insert(vn, Rc::new(Value::const_(42, 8)));

        // into_iter() should yield owned SingleValuation entries
        let mut count = 0;
        for entry in valuation {
            count += 1;
            assert!(matches!(entry.location, Location::Direct(_)));
            assert_eq!(entry.value, Value::const_(42, 8));
        }
        assert_eq!(count, 1);
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut valuation = ValuationSet::new();
        assert_eq!(valuation.len(), 0);
        assert!(valuation.is_empty());

        let vn = VarNode::new(0x1000, 8u32, 0u32);
        valuation.direct_writes.insert(vn, Rc::new(Value::const_(42, 8)));

        assert_eq!(valuation.len(), 1);
        assert!(!valuation.is_empty());

        // Add an indirect write (key must be a Load expression)
        let load_key = Value::Load(crate::analysis::valuation::simple::value::Load(
            Rc::new(Value::const_(100, 8)),
            8,
            1,
        ));
        valuation
            .indirect_writes
            .insert(load_key, Rc::new(Value::const_(200, 8)));

        assert_eq!(valuation.len(), 2);
        assert!(!valuation.is_empty());
    }

    #[test]
    fn test_keys_iterator() {
        let mut valuation = ValuationSet::new();
        let vn1 = VarNode::new(0x1000, 8u32, 0u32);
        let vn2 = VarNode::new(0x2000, 8u32, 0u32);

        valuation.direct_writes.insert(vn1, Rc::new(Value::const_(42, 8)));
        valuation.direct_writes.insert(vn2, Rc::new(Value::const_(99, 8)));

        let keys: Vec<_> = valuation.keys().collect();
        assert_eq!(keys.len(), 2);
        for key in keys {
            assert!(matches!(key, Location::Direct(_)));
        }
    }

    #[test]
    fn test_values_iterator() {
        let mut valuation = ValuationSet::new();
        let vn1 = VarNode::new(0x1000, 8u32, 0u32);
        let vn2 = VarNode::new(0x2000, 8u32, 0u32);

        valuation.direct_writes.insert(vn1, Rc::new(Value::const_(42, 8)));
        valuation.direct_writes.insert(vn2, Rc::new(Value::const_(99, 8)));

        let values: Vec<_> = valuation.values().collect();
        assert_eq!(values.len(), 2);
        assert!(values.contains(&&Value::const_(42, 8)));
        assert!(values.contains(&&Value::const_(99, 8)));
    }

    #[test]
    fn test_values_mut_iterator() {
        let mut valuation = ValuationSet::new();
        let vn = VarNode::new(0x1000, 8u32, 0u32);

        valuation.direct_writes.insert(vn, Rc::new(Value::const_(42, 8)));

        // Mutate all values
        for val in valuation.values_mut() {
            *val = Rc::new(Value::const_(1000, 8));
        }

        // Verify mutation worked
        assert_eq!(
            valuation.get(Location::Direct(vn)),
            Some(&Value::const_(1000, 8))
        );
    }

    #[test]
    fn test_display() {
        let mut valuation = ValuationSet::new();
        let vn = VarNode::new(0x1000, 8u32, 0u32);
        valuation.direct_writes.insert(vn, Rc::new(Value::const_(42, 8)));

        let display_str = format!("{}", valuation);
        assert!(display_str.starts_with("Valuation {"));
        assert!(display_str.contains("="));
        assert!(display_str.ends_with("}"));
    }
    
    #[test]
    fn sub_register_write_updates_parent() {
        let rax = VarNode::new(0x0u64, 8u32, 0u32);
        let al = VarNode::new(0x0u64, 1u32, 0u32);

        let mut vs = ValuationSet::new();
        vs.add(rax, Value::const_(0x1122334455667788_u64 as i64, 8));
        vs.add(al, Value::const_(0x49, 1));

        // RAX low byte replaced: 0x11223344556677_88 → 0x11223344556677_49
        let rax_val = vs.direct_writes.get(rax).expect("RAX must be present");
        assert_eq!(**rax_val, Value::const_(0x1122334455667749_u64 as i64, 8));

        let al_val = vs.direct_writes.get(al).expect("AL must be present");
        assert_eq!(**al_val, Value::const_(0x49, 1));
    }

    #[test]
    fn sub_register_write_at_nonzero_byte_offset_updates_parent() {
        // parent at offset 0 size 4, child at offset 1 size 1 (second byte)
        let parent = VarNode::new(0x0u64, 4u32, 0u32);
        let child = VarNode::new(0x1u64, 1u32, 0u32);

        let mut vs = ValuationSet::new();
        vs.add(parent, Value::const_(0xAABBCCDD_u64 as i64, 4));
        vs.add(child, Value::const_(0x49, 1));

        // insert_bytes at byte_offset=1: 0xAABBCCDD & 0xFFFF00FF | 0x4900 = 0xAABB49DD
        let parent_val = vs
            .direct_writes
            .get(parent)
            .expect("parent must be present");
        assert_eq!(**parent_val, Value::const_(0xAABB49DD_u64 as i64, 4));
    }

    #[test]
    fn sub_register_write_with_symbolic_parent_stores_expression() {
        let rax_vn = VarNode::new(0x100u64, 8u32, 0u32);
        let rbx_vn = VarNode::new(0x200u64, 8u32, 0u32);
        let al_vn = VarNode::new(0x100u64, 1u32, 0u32);

        let mut vs = ValuationSet::new();
        vs.add(rax_vn, Value::entry(rbx_vn));
        vs.add(al_vn, Value::const_(0x49, 1));

        let rax_val = vs.direct_writes.get(rax_vn).expect("RAX must be present");
        assert!(
            rax_val.as_or().is_some(),
            "expected Or expression when parent was symbolic, got: {:?}",
            rax_val
        );
        assert_eq!(rax_val.size(), 8);
    }

    #[test]
    fn full_register_overwrite_does_not_trigger_upward_propagation() {
        let rax = VarNode::new(0x0u64, 8u32, 0u32);

        let mut vs = ValuationSet::new();
        vs.add(rax, Value::const_(0xDEAD, 8));
        vs.add(rax, Value::const_(0x1234, 8));

        assert_eq!(vs.direct_writes.len(), 1);
        assert_eq!(vs.get(Location::Direct(rax)), Some(&Value::const_(0x1234, 8)));
    }
}
