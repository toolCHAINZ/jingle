use std::borrow::Borrow;
use std::collections::BTreeMap;

use crate::display::JingleDisplay;
use jingle_sleigh::{SleighArchInfo, VarNode};
use std::fmt::{Display, Formatter};

use crate::analysis::{valuation::Value, varnode_map::VarNodeMap};

/// A container holding both direct writes (varnode -> value) and indirect writes
/// ([pointer expression] -> value) produced by stores.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ValuationSet {
    pub direct_writes: VarNodeMap<Value>,
    /// Keyed on the load expression representing the memory location (e.g. `Load(ptr, size)`),
    /// not the raw pointer. This matches the `Value::Load` representation used when the
    /// stored value is read back by a load operation.
    /// Note: for now we are making the simplifying assumption that all indirect writes happen
    /// in one space; this map can be keyed by both `Value` and `SpaceIndex` to generalize.
    pub indirect_writes: BTreeMap<Value, Value>,
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
        direct_writes: VarNodeMap<Value>,
        indirect_writes: BTreeMap<Value, Value>,
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
            Location::Direct(vn) => self.direct_writes.get(vn),
            Location::Indirect(ptr_intern) => {
                // indirect_writes keyed by Value, lookup by reference to the Value
                self.indirect_writes.get(ptr_intern)
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

    /// Interpret this valuation in terms of another valuation (context).
    ///
    /// Recursively substitutes `Entry` and Symbolic values in this valuation using
    /// the mappings from the context valuation. This enables compositional reasoning:
    /// if `self` states `RAX = RBX` and `context` states `RBX = RCX`, then
    /// `self.assuming(&context)` produces `RAX = RCX`.
    ///
    /// For indirect (memory) locations, both the location key and the value are
    /// substituted. For example, if `self` has `[Load(RSP + 4)] = 8` and `context`
    /// has `Load(RSP + 4) = 0xdeadbeef`, then the result has `[0xdeadbeef] = 8`.
    ///
    /// Cycles are handled by simplification: if `A = B` in self and `B = A` in context,
    /// the result simplifies to `A = A`.
    ///
    /// # Example
    /// ```ignore
    /// let mut val1 = ValuationSet::new();
    /// val1.add(rax, Value::entry(rbx));  // RAX = RBX
    ///
    /// let mut context = ValuationSet::new();
    /// context.add(rbx, Value::entry(rcx));  // RBX = RCX
    ///
    /// let result = val1.assuming(&context);
    /// // result contains: RAX = RCX
    /// ```
    pub fn assuming(&self, context: &ValuationSet) -> ValuationSet {
        let mut result = ValuationSet::new();

        // Substitute all direct writes
        for (vn, value) in self.direct_writes.items() {
            let substituted_value = value.substitute(context);
            result.add(*vn, substituted_value);
        }

        // Substitute all indirect writes
        // Both the key (symbolic expression) and the value need substitution
        for (sym_expr, value) in &self.indirect_writes {
            let substituted_key = sym_expr.substitute(context);
            let substituted_value = value.substitute(context);
            result.add(substituted_key, substituted_value);
        }

        result
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
    /// Values are simplified before insertion to keep stored representations normalized.
    pub fn add<L, V>(&mut self, loc: L, value: V)
    where
        L: Into<Location>,
        V: Into<Value>,
    {
        let loc = loc.into();
        let val = value.into().simplify();
        match loc {
            Location::Direct(vn) => {
                // Remove any existing entries whose range is entirely covered by this write.
                // Writing to a larger region (e.g. register[4:8]) physically overwrites all
                // sub-regions (e.g. register[4:4]) that fall within it.
                self.direct_writes
                    .retain(|existing, _| !vn.covers(existing) || existing == &vn);
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
    direct_iter: crate::analysis::varnode_map::Iter<'a, Value>,
    indirect_iter: std::collections::btree_map::Iter<'a, Value, Value>,
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
    direct_iter: crate::analysis::varnode_map::IterMut<'a, Value>,
    indirect_iter: std::collections::btree_map::IterMut<'a, Value, Value>,
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
    type Item = (Location, &'a mut Value);

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
    direct_iter: crate::analysis::varnode_map::Iter<'a, Value>,
    indirect_iter: std::collections::btree_map::Iter<'a, Value, Value>,
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
    direct_iter: crate::analysis::varnode_map::Iter<'a, Value>,
    indirect_iter: std::collections::btree_map::Iter<'a, Value, Value>,
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

/// A mutable iterator over the values of a `ValuationSet`.
///
/// This struct is created by the `values_mut` method on `ValuationSet`.
pub struct ValuesMut<'a> {
    direct_iter: crate::analysis::varnode_map::IterMut<'a, Value>,
    indirect_iter: std::collections::btree_map::IterMut<'a, Value, Value>,
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
    type Item = &'a mut Value;

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
    direct_entries: std::vec::IntoIter<(VarNode, Value)>,
    indirect_entries: std::vec::IntoIter<(Value, Value)>,
    direct_done: bool,
}

impl Iterator for ValuationIntoIter {
    type Item = Valuation;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.direct_done {
            if let Some((vn, val)) = self.direct_entries.next() {
                return Some(Valuation::new_direct(vn, val));
            }
            self.direct_done = true;
        }
        self.indirect_entries
            .next()
            .map(|(ptr, val)| Valuation::new_indirect(ptr, val))
    }
}

impl<'a> IntoIterator for &'a mut ValuationSet {
    type Item = (Location, &'a mut Value);
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
            write!(f, "{} = {}", vn, val)?;
        }

        // Indirect writes ([ptr_expr] -> val)
        for (ptr, val) in &self.indirect_writes {
            if !first {
                write!(f, ", ")?;
            }
            first = false;
            write!(f, "[{}] = {}", ptr, val)?;
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
        let mut valuation = ValuationSet::new();
        let vn = VarNode::new(0x1000, 8u32, 0u32);
        valuation.direct_writes.insert(vn, Value::const_(42));

        // iter() should yield (location, &value) tuples
        let mut count = 0;
        for (loc, val) in valuation.iter() {
            count += 1;
            assert!(matches!(loc, Location::Direct(_)));
            assert_eq!(*val, Value::const_(42));
        }
        assert_eq!(count, 1);
    }

    #[test]
    fn test_iter_mut_yields_tuples() {
        let mut valuation = ValuationSet::new();
        let vn = VarNode::new(0x1000, 8u32, 0u32);
        valuation.direct_writes.insert(vn, Value::const_(42));

        // iter_mut() should yield (location, &mut value) tuples
        for (loc, val) in valuation.iter_mut() {
            assert!(matches!(loc, Location::Direct(_)));
            *val = Value::const_(100);
        }

        // Verify mutation worked
        assert_eq!(valuation.direct_writes.get(vn), Some(&Value::const_(100)));
    }

    #[test]
    fn test_into_iter_yields_entries() {
        let mut valuation = ValuationSet::new();
        let vn = VarNode::new(0x1000, 8u32, 0u32);
        valuation.direct_writes.insert(vn, Value::const_(42));

        // into_iter() should yield owned SingleValuation entries
        let mut count = 0;
        for entry in valuation {
            count += 1;
            assert!(matches!(entry.location, Location::Direct(_)));
            assert_eq!(entry.value, Value::const_(42));
        }
        assert_eq!(count, 1);
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut valuation = ValuationSet::new();
        assert_eq!(valuation.len(), 0);
        assert!(valuation.is_empty());

        let vn = VarNode::new(0x1000, 8u32, 0u32);
        valuation.direct_writes.insert(vn, Value::const_(42));

        assert_eq!(valuation.len(), 1);
        assert!(!valuation.is_empty());

        // Add an indirect write (key must be a Load expression)
        let load_key = Value::Load(crate::analysis::valuation::simple::value::Load(
            internment::Intern::new(Value::const_(100)),
            8,
        ));
        valuation
            .indirect_writes
            .insert(load_key, Value::const_(200));

        assert_eq!(valuation.len(), 2);
        assert!(!valuation.is_empty());
    }

    #[test]
    fn test_keys_iterator() {
        let mut valuation = ValuationSet::new();
        let vn1 = VarNode::new(0x1000, 8u32, 0u32);
        let vn2 = VarNode::new(0x2000, 8u32, 0u32);

        valuation.direct_writes.insert(vn1, Value::const_(42));
        valuation.direct_writes.insert(vn2, Value::const_(99));

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

        valuation.direct_writes.insert(vn1, Value::const_(42));
        valuation.direct_writes.insert(vn2, Value::const_(99));

        let values: Vec<_> = valuation.values().collect();
        assert_eq!(values.len(), 2);
        assert!(values.contains(&&Value::const_(42)));
        assert!(values.contains(&&Value::const_(99)));
    }

    #[test]
    fn test_values_mut_iterator() {
        let mut valuation = ValuationSet::new();
        let vn = VarNode::new(0x1000, 8u32, 0u32);

        valuation.direct_writes.insert(vn, Value::const_(42));

        // Mutate all values
        for val in valuation.values_mut() {
            *val = Value::const_(1000);
        }

        // Verify mutation worked
        assert_eq!(valuation.direct_writes.get(vn), Some(&Value::const_(1000)));
    }

    #[test]
    fn test_display() {
        let mut valuation = ValuationSet::new();
        let vn = VarNode::new(0x1000, 8u32, 0u32);
        valuation.direct_writes.insert(vn, Value::const_(42));

        let display_str = format!("{}", valuation);
        assert!(display_str.starts_with("Valuation {"));
        assert!(display_str.contains("="));
        assert!(display_str.ends_with("}"));
    }

    #[test]
    fn test_assuming_direct_substitution() {
        // Test: RAX = RBX, assuming RBX = RCX, should produce RAX = RCX
        let rax = VarNode::new(0x1000, 8u32, 0u32);
        let rbx = VarNode::new(0x2000, 8u32, 0u32);
        let rcx = VarNode::new(0x3000, 8u32, 0u32);

        let mut val1 = ValuationSet::new();
        val1.add(rax, Value::entry(rbx));

        let mut context = ValuationSet::new();
        context.add(rbx, Value::entry(rcx));

        let result = val1.assuming(&context);

        // Should have RAX = RCX
        assert_eq!(result.direct_writes.get(rax), Some(&Value::entry(rcx)));
    }

    #[test]
    fn test_assuming_chained_substitution() {
        // Test: RAX = RBX, assuming RBX = RCX and RCX = RDX, should produce RAX = RDX
        let rax = VarNode::new(0x1000, 8u32, 0u32);
        let rbx = VarNode::new(0x2000, 8u32, 0u32);
        let rcx = VarNode::new(0x3000, 8u32, 0u32);
        let rdx = VarNode::new(0x4000, 8u32, 0u32);

        let mut val1 = ValuationSet::new();
        val1.add(rax, Value::entry(rbx));

        let mut context = ValuationSet::new();
        context.add(rbx, Value::entry(rcx));
        context.add(rcx, Value::entry(rdx));

        let result = val1.assuming(&context);

        // Should have RAX = RDX (chained substitution)
        assert_eq!(result.direct_writes.get(rax), Some(&Value::entry(rdx)));
    }

    #[test]
    fn test_assuming_cycle_handling() {
        // Test: RAX = RBX, assuming RBX = RAX, should produce RAX = RAX (simplified)
        let rax = VarNode::new(0x1000, 8u32, 0u32);
        let rbx = VarNode::new(0x2000, 8u32, 0u32);

        let mut val1 = ValuationSet::new();
        val1.add(rax, Value::entry(rbx));

        let mut context = ValuationSet::new();
        context.add(rbx, Value::entry(rax));

        let result = val1.assuming(&context);

        // After substitution and simplification, should have RAX = RAX
        // But since RAX = RAX is tautological, the add() method might optimize it
        // Let's just check what we get
        let val = result.direct_writes.get(rax);
        assert!(val.is_some());
        // It should be Entry(rax) after substituting Entry(rbx) with Entry(rax)
        assert_eq!(val, Some(&Value::entry(rax)));
    }

    #[test]
    fn test_assuming_indirect_pointer_substitution() {
        // Test: [Load(RSP+4)] = 8, assuming RSP = 0x1000
        // Should produce: [Load(0x1004)] = 8
        let rsp = VarNode::new(0x1000, 8u32, 0u32);

        // Create Load(RSP + 4)
        let rsp_plus_4 = Value::entry(rsp) + Value::const_(4);
        let load_expr = Value::Load(crate::analysis::valuation::simple::value::Load(
            internment::Intern::new(rsp_plus_4.clone()),
            8,
        ));

        let mut val1 = ValuationSet::new();
        // Add indirect write: [Load(RSP+4)] = 8
        val1.add(load_expr.clone(), Value::const_(8));

        let mut context = ValuationSet::new();
        // Add direct write: RSP = 0x1000
        context.add(rsp, Value::const_(0x1000));

        let result = val1.assuming(&context);

        // The load expression should be substituted:
        // Load(RSP+4) where RSP is Entry(rsp)
        // Substitute: Entry(rsp) -> 0x1000
        // So RSP+4 -> 0x1000 + 4 -> 0x1004 (after simplification)
        // So Load(RSP+4) -> Load(0x1004)
        // Result should have [Load(0x1004)] = 8

        let expected_key = Value::Load(crate::analysis::valuation::simple::value::Load(
            internment::Intern::new(Value::const_(0x1004)),
            8,
        ));

        assert_eq!(
            result.indirect_writes.get(&expected_key),
            Some(&Value::const_(8))
        );
    }

    #[test]
    fn test_assuming_indirect_value_substitution() {
        // Test: [Load(0x1000)] = RBX, assuming RBX = 42
        // Should produce: [Load(0x1000)] = 42
        let rbx = VarNode::new(0x2000, 8u32, 0u32);

        let load_expr = Value::Load(crate::analysis::valuation::simple::value::Load(
            internment::Intern::new(Value::const_(0x1000)),
            8,
        ));

        let mut val1 = ValuationSet::new();
        // Add indirect write: [Load(0x1000)] = RBX
        val1.add(load_expr.clone(), Value::entry(rbx));

        let mut context = ValuationSet::new();
        // Add direct write: RBX = 42
        context.add(rbx, Value::const_(42));

        let result = val1.assuming(&context);

        // The value should be substituted:
        // Entry(rbx) -> 42
        // Result should have [Load(0x1000)] = 42

        assert_eq!(
            result.indirect_writes.get(&load_expr),
            Some(&Value::const_(42))
        );
    }

    #[test]
    fn test_assuming_no_context() {
        // Test: RAX = RBX with empty context should produce RAX = RBX unchanged
        let rax = VarNode::new(0x1000, 8u32, 0u32);
        let rbx = VarNode::new(0x2000, 8u32, 0u32);

        let mut val1 = ValuationSet::new();
        val1.add(rax, Value::entry(rbx));

        let context = ValuationSet::new();

        let result = val1.assuming(&context);

        // Should have RAX = RBX unchanged
        assert_eq!(result.direct_writes.get(rax), Some(&Value::entry(rbx)));
    }

    #[test]
    fn test_assuming_expression_substitution() {
        // Test: RAX = RBX + 4, assuming RBX = 10, should produce RAX = 14
        let rax = VarNode::new(0x1000, 8u32, 0u32);
        let rbx = VarNode::new(0x2000, 8u32, 0u32);

        let mut val1 = ValuationSet::new();
        val1.add(rax, Value::entry(rbx) + Value::const_(4));

        let mut context = ValuationSet::new();
        context.add(rbx, Value::const_(10));

        let result = val1.assuming(&context);

        // Should have RAX = 14 (simplified from 10 + 4)
        assert_eq!(result.direct_writes.get(rax), Some(&Value::const_(14)));
    }

    #[test]
    fn test_assuming_top_propagation() {
        // Test: RAX = RBX, assuming RBX = Top, should produce RAX = Top
        let rax = VarNode::new(0x1000, 8u32, 0u32);
        let rbx = VarNode::new(0x2000, 8u32, 0u32);

        let mut val1 = ValuationSet::new();
        val1.add(rax, Value::entry(rbx));

        let mut context = ValuationSet::new();
        context.add(rbx, Value::Top);

        let result = val1.assuming(&context);

        // Should have RAX = Top
        assert_eq!(result.direct_writes.get(rax), Some(&Value::Top));
    }

    #[test]
    fn test_assuming_multiple_entries() {
        // Test multiple valuations being substituted at once
        let rax = VarNode::new(0x1000, 8u32, 0u32);
        let rbx = VarNode::new(0x2000, 8u32, 0u32);
        let rcx = VarNode::new(0x3000, 8u32, 0u32);
        let rdx = VarNode::new(0x4000, 8u32, 0u32);

        let mut val1 = ValuationSet::new();
        val1.add(rax, Value::entry(rbx));
        val1.add(rcx, Value::entry(rdx));

        let mut context = ValuationSet::new();
        context.add(rbx, Value::const_(100));
        context.add(rdx, Value::const_(200));

        let result = val1.assuming(&context);

        assert_eq!(result.direct_writes.get(rax), Some(&Value::const_(100)));
        assert_eq!(result.direct_writes.get(rcx), Some(&Value::const_(200)));
    }
}
