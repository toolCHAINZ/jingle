use std::borrow::Borrow;

use crate::display::JingleDisplay;
use im::OrdMap;
use jingle_sleigh::{SleighArchInfo, VarNode};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::rc::Rc;

use crate::analysis::valuation::simple::value::{IntoRcValue, Load};
use crate::analysis::{valuation::Value, varnode_map::VarNodeMap};

// Serialize OrdMap as a Vec of (key, value) tuples for format stability.
mod ordmap_as_vec {
    use im::OrdMap;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<K, V, S>(map: &OrdMap<K, V>, s: S) -> Result<S::Ok, S::Error>
    where
        K: Serialize + Ord + Clone,
        V: Serialize + Clone,
        S: Serializer,
    {
        map.iter().collect::<Vec<_>>().serialize(s)
    }

    pub fn deserialize<'de, K, V, D>(d: D) -> Result<OrdMap<K, V>, D::Error>
    where
        K: Deserialize<'de> + Ord + Clone,
        V: Deserialize<'de> + Clone,
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
    #[serde(with = "ordmap_as_vec")]
    pub indirect_writes: OrdMap<Value, VarNodeMap<Rc<Value>>>,
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
            indirect_writes: OrdMap::new(),
        }
    }

    /// Construct a `ValuationSet` with the provided direct and indirect write maps.
    ///
    /// This allows callers to build a `ValuationSet` with pre-populated contents
    /// instead of creating an empty one and inserting entries afterwards.
    pub fn with_contents(
        direct_writes: VarNodeMap<Rc<Value>>,
        indirect_writes: OrdMap<Value, VarNodeMap<Rc<Value>>>,
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
                if let Value::Load(Load(ptr, size, space)) = ptr_intern {
                    let vn = VarNode::new(0, *size as u32, u32::from(*space));
                    self.indirect_writes
                        .get(ptr.as_ref())?
                        .get(vn)
                        .map(|rc| rc.as_ref())
                } else {
                    None
                }
            }
        }
    }

    /// Returns the number of entries (both direct and indirect) in this valuation.
    pub fn len(&self) -> usize {
        let indirect: usize = self.indirect_writes.iter().map(|(_, m)| m.len()).sum();
        self.direct_writes.len() + indirect
    }

    /// Returns `true` if this valuation contains no entries.
    pub fn is_empty(&self) -> bool {
        self.direct_writes.is_empty() && self.indirect_writes.iter().all(|(_, m)| m.is_empty())
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

    pub fn iter(&self) -> ValuationIter<'_> {
        self.into_iter()
    }

    pub fn remove_value_from(&mut self, loc: &Location) {
        match loc {
            Location::Direct(vn) => {
                self.direct_writes.remove(vn);
            }
            Location::Indirect(ptr_intern) => {
                if let Value::Load(Load(ptr, size, space)) = ptr_intern {
                    let vn = VarNode::new(0, *size as u32, u32::from(*space));
                    if let Some(inner) = self.indirect_writes.get_mut(ptr.as_ref()) {
                        inner.remove(vn);
                        if inner.is_empty() {
                            self.indirect_writes.remove(ptr.as_ref());
                        }
                    }
                }
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
                        let merged = Value::insert_bytes(parent_val, Rc::clone(&val), byte_offset);
                        (*parent_vn, Value::simplify_shared(&Rc::new(merged)))
                    })
                    .collect();

                for (parent_vn, merged_val) in parents {
                    self.direct_writes.insert(parent_vn, merged_val);
                }

                self.direct_writes.insert(vn, val);
            }
            Location::Indirect(ptr_intern) => {
                if let Value::Load(Load(ptr, new_size, space)) = &ptr_intern {
                    let new_vn = VarNode::new(0, *new_size as u32, u32::from(*space));
                    if self.indirect_writes.get(ptr.as_ref()).is_none() {
                        self.indirect_writes
                            .insert(ptr.as_ref().clone(), VarNodeMap::new());
                    }
                    let inner = self
                        .indirect_writes
                        .get_mut(ptr.as_ref())
                        .expect("just inserted");
                    inner.retain(|existing, _| !new_vn.covers(existing) || existing == &new_vn);
                    let parents: Vec<(VarNode, Rc<Value>)> = inner
                        .items()
                        .filter(|(existing, _)| existing.covers(&new_vn) && *existing != &new_vn)
                        .map(|(parent_vn, parent_val)| {
                            let inner_off = (new_vn.offset() - parent_vn.offset()) as usize;
                            let merged =
                                Value::insert_bytes(parent_val, Rc::clone(&val), inner_off);
                            (*parent_vn, Value::simplify_shared(&Rc::new(merged)))
                        })
                        .collect();
                    for (parent_vn, merged_val) in parents {
                        inner.insert(parent_vn, merged_val);
                    }
                    inner.insert(new_vn, val);
                }
                // Non-Load indirect: no-op.
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
    indirect_outer: im::ordmap::Iter<'a, Value, VarNodeMap<Rc<Value>>>,
    indirect_base: Option<&'a Value>,
    indirect_inner: Option<crate::analysis::varnode_map::Iter<'a, Rc<Value>>>,
    direct_done: bool,
}

impl<'a> ValuationIter<'a> {
    pub fn new(valuation: &'a ValuationSet) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter(),
            indirect_outer: valuation.indirect_writes.iter(),
            indirect_base: None,
            indirect_inner: None,
            direct_done: false,
        }
    }
}

impl<'a> Iterator for ValuationIter<'a> {
    type Item = (Location, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.direct_done {
            if let Some((vn, val)) = self.direct_iter.next() {
                return Some((Location::Direct(*vn), val.as_ref()));
            }
            self.direct_done = true;
        }
        loop {
            if let Some(ref mut inner) = self.indirect_inner {
                if let Some((vn, val)) = inner.next() {
                    let base = self.indirect_base.expect("base set with inner");
                    let load = Value::load(base.clone(), vn.size(), vn.space_index() as u8);
                    return Some((Location::Indirect(load), val.as_ref()));
                }
            }
            match self.indirect_outer.next() {
                Some((ptr, inner_map)) => {
                    self.indirect_base = Some(ptr);
                    self.indirect_inner = Some(inner_map.items());
                }
                None => return None,
            }
        }
    }
}

impl<'a> IntoIterator for &'a ValuationSet {
    type Item = (Location, &'a Value);
    type IntoIter = ValuationIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ValuationIter::new(self)
    }
}

/// An iterator over the keys (locations) of a `ValuationSet`.
///
/// This struct is created by the `keys` method on `ValuationSet`.
pub struct Keys<'a> {
    direct_iter: crate::analysis::varnode_map::Iter<'a, Rc<Value>>,
    indirect_outer: im::ordmap::Iter<'a, Value, VarNodeMap<Rc<Value>>>,
    indirect_base: Option<&'a Value>,
    indirect_inner: Option<crate::analysis::varnode_map::Iter<'a, Rc<Value>>>,
    direct_done: bool,
}

impl<'a> Keys<'a> {
    pub fn new(valuation: &'a ValuationSet) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter(),
            indirect_outer: valuation.indirect_writes.iter(),
            indirect_base: None,
            indirect_inner: None,
            direct_done: false,
        }
    }
}

impl<'a> Iterator for Keys<'a> {
    type Item = Location;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.direct_done {
            if let Some((vn, _)) = self.direct_iter.next() {
                return Some(Location::Direct(*vn));
            }
            self.direct_done = true;
        }
        loop {
            if let Some(ref mut inner) = self.indirect_inner {
                if let Some((vn, _)) = inner.next() {
                    let base = self.indirect_base.expect("base set with inner");
                    let load = Value::load(base.clone(), vn.size(), vn.space_index() as u8);
                    return Some(Location::Indirect(load));
                }
            }
            match self.indirect_outer.next() {
                Some((ptr, inner_map)) => {
                    self.indirect_base = Some(ptr);
                    self.indirect_inner = Some(inner_map.items());
                }
                None => return None,
            }
        }
    }
}

/// An iterator over the values of a `ValuationSet`.
///
/// This struct is created by the `values` method on `ValuationSet`.
pub struct Values<'a> {
    direct_iter: crate::analysis::varnode_map::Iter<'a, Rc<Value>>,
    indirect_outer: im::ordmap::Iter<'a, Value, VarNodeMap<Rc<Value>>>,
    indirect_inner: Option<crate::analysis::varnode_map::Iter<'a, Rc<Value>>>,
    direct_done: bool,
}

impl<'a> Values<'a> {
    pub fn new(valuation: &'a ValuationSet) -> Self {
        Self {
            direct_iter: valuation.direct_writes.iter(),
            indirect_outer: valuation.indirect_writes.iter(),
            indirect_inner: None,
            direct_done: false,
        }
    }
}

impl<'a> Iterator for Values<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.direct_done {
            if let Some((_, val)) = self.direct_iter.next() {
                return Some(val.as_ref());
            }
            self.direct_done = true;
        }
        loop {
            if let Some(ref mut inner) = self.indirect_inner {
                if let Some((_, val)) = inner.next() {
                    return Some(val.as_ref());
                }
            }
            match self.indirect_outer.next() {
                Some((_, inner_map)) => {
                    self.indirect_inner = Some(inner_map.items());
                }
                None => return None,
            }
        }
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
        self.indirect_entries.next().map(|(load_key, val)| {
            let owned = Rc::try_unwrap(val).unwrap_or_else(|rc| (*rc).clone());
            Valuation::new_indirect(load_key, owned)
        })
    }
}

impl IntoIterator for ValuationSet {
    type Item = Valuation;
    type IntoIter = ValuationIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        let indirect_entries: Vec<(Value, Rc<Value>)> = self
            .indirect_writes
            .into_iter()
            .flat_map(|(base, inner_map)| {
                let base_rc = std::rc::Rc::new(base);
                inner_map
                    .into_iter()
                    .map(move |(vn, val)| {
                        let load_key = Value::load(
                            std::rc::Rc::clone(&base_rc),
                            vn.size(),
                            vn.space_index() as u8,
                        );
                        (load_key, val)
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        ValuationIntoIter {
            direct_entries: self
                .direct_writes
                .into_iter()
                .collect::<Vec<_>>()
                .into_iter(),
            indirect_entries: indirect_entries.into_iter(),
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
        for (ptr, inner_map) in &self.indirect_writes {
            for (vn, val) in inner_map.items() {
                if !first {
                    write!(f, ", ")?;
                }
                first = false;
                let load_key = Value::load(ptr.clone(), vn.size(), vn.space_index() as u8);
                write!(f, "[{}] = {}", load_key, val.as_ref())?;
            }
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
        for (ptr, inner_map) in &self.indirect_writes {
            for (vn, val) in inner_map.items() {
                if !first {
                    write!(f, ", ")?;
                }
                first = false;
                let load_key = Value::load(ptr.clone(), vn.size(), vn.space_index() as u8);
                write!(
                    f,
                    "[{}] = {}",
                    load_key.display(info),
                    val.as_ref().display(info)
                )?;
            }
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
        valuation
            .direct_writes
            .insert(vn, Rc::new(Value::const_(42, 8)));

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
    fn test_into_iter_yields_entries() {
        let mut valuation = ValuationSet::new();
        let vn = VarNode::new(0x1000, 8u32, 0u32);
        valuation
            .direct_writes
            .insert(vn, Rc::new(Value::const_(42, 8)));

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
        valuation
            .direct_writes
            .insert(vn, Rc::new(Value::const_(42, 8)));

        assert_eq!(valuation.len(), 1);
        assert!(!valuation.is_empty());

        // Add an indirect write via add() so the two-level structure is populated correctly.
        let load_key = Value::load(Value::const_(100, 8), 8, 1);
        valuation.add(load_key, Value::const_(200, 8));

        assert_eq!(valuation.len(), 2);
        assert!(!valuation.is_empty());
    }

    #[test]
    fn test_keys_iterator() {
        let mut valuation = ValuationSet::new();
        let vn1 = VarNode::new(0x1000, 8u32, 0u32);
        let vn2 = VarNode::new(0x2000, 8u32, 0u32);

        valuation
            .direct_writes
            .insert(vn1, Rc::new(Value::const_(42, 8)));
        valuation
            .direct_writes
            .insert(vn2, Rc::new(Value::const_(99, 8)));

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

        valuation
            .direct_writes
            .insert(vn1, Rc::new(Value::const_(42, 8)));
        valuation
            .direct_writes
            .insert(vn2, Rc::new(Value::const_(99, 8)));

        let values: Vec<_> = valuation.values().collect();
        assert_eq!(values.len(), 2);
        assert!(values.contains(&&Value::const_(42, 8)));
        assert!(values.contains(&&Value::const_(99, 8)));
    }

    #[test]
    fn test_display() {
        let mut valuation = ValuationSet::new();
        let vn = VarNode::new(0x1000, 8u32, 0u32);
        valuation
            .direct_writes
            .insert(vn, Rc::new(Value::const_(42, 8)));

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
    fn indirect_larger_write_removes_smaller() {
        let ptr = Value::entry(VarNode::new(0x1000u64, 8u32, 0u32));
        let mut vs = ValuationSet::new();
        vs.add(Value::load(ptr.clone(), 2, 1), Value::const_(0xABCD, 2));
        vs.add(
            Value::load(ptr.clone(), 8, 1),
            Value::const_(0x1122334455667788_u64 as i64, 8),
        );

        // The 8-byte write covers the 2-byte entry; only the 8-byte entry survives.
        let inner = vs.indirect_writes.get(&ptr).expect("ptr must be a key");
        assert_eq!(inner.len(), 1, "only one entry should remain");
        let vn8 = jingle_sleigh::VarNode::new(0u64, 8u32, 1u32);
        assert!(inner.contains(vn8), "the 8-byte entry must survive");
    }

    #[test]
    fn indirect_smaller_write_splices_parent() {
        let ptr = Value::entry(VarNode::new(0x1000u64, 8u32, 0u32));
        let mut vs = ValuationSet::new();
        vs.add(
            Value::load(ptr.clone(), 8, 1),
            Value::const_(0x1122334455667788_u64 as i64, 8),
        );
        vs.add(Value::load(ptr.clone(), 2, 1), Value::const_(0x4949, 2));

        // The 2-byte write splices into the 8-byte parent at offset 0.
        let inner = vs.indirect_writes.get(&ptr).expect("ptr must be a key");
        let vn8 = jingle_sleigh::VarNode::new(0u64, 8u32, 1u32);
        let vn2 = jingle_sleigh::VarNode::new(0u64, 2u32, 1u32);
        assert!(inner.contains(vn8), "8-byte entry must survive");
        assert!(inner.contains(vn2), "2-byte entry must survive");

        let expected = Value::insert_bytes(
            Value::const_(0x1122334455667788_u64 as i64, 8),
            Value::const_(0x4949, 2),
            0,
        )
        .simplify();
        let actual = inner.get(vn8).expect("8-byte entry present");
        assert_eq!(**actual, expected);
    }

    #[test]
    fn indirect_same_size_overwrites() {
        let ptr = Value::entry(VarNode::new(0x1000u64, 8u32, 0u32));
        let mut vs = ValuationSet::new();
        vs.add(Value::load(ptr.clone(), 4, 1), Value::const_(0xDEAD, 4));
        vs.add(Value::load(ptr.clone(), 4, 1), Value::const_(0x1234, 4));

        let inner = vs.indirect_writes.get(&ptr).expect("ptr must be a key");
        assert_eq!(inner.len(), 1);
        let vn4 = jingle_sleigh::VarNode::new(0u64, 4u32, 1u32);
        assert_eq!(**inner.get(vn4).unwrap(), Value::const_(0x1234, 4));
    }

    #[test]
    fn full_register_overwrite_does_not_trigger_upward_propagation() {
        let rax = VarNode::new(0x0u64, 8u32, 0u32);

        let mut vs = ValuationSet::new();
        vs.add(rax, Value::const_(0xDEAD, 8));
        vs.add(rax, Value::const_(0x1234, 8));

        assert_eq!(vs.direct_writes.len(), 1);
        assert_eq!(
            vs.get(Location::Direct(rax)),
            Some(&Value::const_(0x1234, 8))
        );
    }
}
