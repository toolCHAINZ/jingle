use std::{borrow::Borrow, cmp::Ordering};

use jingle_sleigh::VarNode;

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
struct VnWrapper(pub VarNode, pub usize);

impl PartialOrd for VnWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VnWrapper {
    fn cmp(&self, other: &Self) -> Ordering {
        let s_vn = &self.0;
        let o_vn = &other.0;
        match s_vn.space_index.cmp(&o_vn.space_index) {
            Ordering::Equal => match s_vn.offset.cmp(&o_vn.offset) {
                Ordering::Equal => s_vn.size.cmp(&o_vn.size),
                a => a,
            },
            a => a,
        }
    }
}

/// A compact map keyed by `VarNode`.
///
/// Internally maintains a sorted vector of varnodes (`vns`) and a parallel `data` vector.
/// The two vectors are kept aligned: `data[i]` is the value for `vns[i].0`. The wrapper also
/// stores the index for debugging/inspection but it is updated on structural changes.
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub struct VarNodeMap<T> {
    vns: Vec<VnWrapper>,
    data: Vec<T>,
}

impl<T> VarNodeMap<T> {
    /// Create an empty map.
    pub fn new() -> Self {
        Self {
            vns: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Is the map empty?
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Helper to compare two varnodes with the same ordering used by `VnWrapper`.
    fn cmp_vn(a: &VarNode, b: &VarNode) -> Ordering {
        match a.space_index.cmp(&b.space_index) {
            Ordering::Equal => match a.offset.cmp(&b.offset) {
                Ordering::Equal => a.size.cmp(&b.size),
                a => a,
            },
            a => a,
        }
    }

    /// Find the position of `vn` in `vns`. Returns `Ok(index)` if present, or `Err(insertion_index)`.
    fn position_of<B: Borrow<VarNode>>(&self, vn: B) -> Result<usize, usize> {
        let key = vn.borrow();
        self.vns.binary_search_by(|w| Self::cmp_vn(&w.0, key))
    }

    /// Returns true if the map contains the given varnode.
    pub fn contains<B: Borrow<VarNode>>(&self, vn: B) -> bool {
        self.position_of(vn).is_ok()
    }

    /// Get a reference to a value by varnode.
    pub fn get<B: Borrow<VarNode>>(&self, vn: B) -> Option<&T> {
        match self.position_of(vn) {
            Ok(idx) => self.data.get(idx),
            Err(_) => None,
        }
    }

    /// Get a mutable reference to a value by varnode.
    pub fn get_mut<B: Borrow<VarNode>>(&mut self, vn: B) -> Option<&mut T> {
        match self.position_of(vn) {
            Ok(idx) => self.data.get_mut(idx),
            Err(_) => None,
        }
    }

    /// Insert a value for `vn`. If the key already exists the old value is returned.
    /// Otherwise inserts and returns `None`.
    ///
    /// Preserves the sorted ordering of keys and updates internal indices accordingly.
    pub fn insert(&mut self, vn: VarNode, value: T) -> Option<T> {
        match self.position_of(&vn) {
            Ok(idx) => {
                // replace existing
                Some(std::mem::replace(&mut self.data[idx], value))
            }
            Err(insert_idx) => {
                // insert at position so vns remains sorted
                let wrapper = VnWrapper(vn, insert_idx);
                self.vns.insert(insert_idx, wrapper);
                self.data.insert(insert_idx, value);
                // fix indices in wrappers from insert_idx onward
                for i in insert_idx..self.vns.len() {
                    self.vns[i].1 = i;
                }
                None
            }
        }
    }

    /// Remove a mapping by varnode. Returns the removed value if present.
    pub fn remove<B: Borrow<VarNode>>(&mut self, vn: B) -> Option<T> {
        match self.position_of(vn) {
            Ok(idx) => {
                // remove both vectors at idx and fix indices after
                self.vns.remove(idx);
                let removed = self.data.remove(idx);
                for i in idx..self.vns.len() {
                    self.vns[i].1 = i;
                }
                Some(removed)
            }
            Err(_) => None,
        }
    }

    /// Retain only the entries specified by the predicate. Predicate receives (&VarNode, &T).
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&VarNode, &T) -> bool,
    {
        // Collect keys to remove to avoid mutating while iterating
        let mut to_remove: Vec<VarNode> = Vec::new();
        for (k, v) in self.items() {
            if !f(k, v) {
                to_remove.push(k.clone());
            }
        }
        for k in to_remove {
            self.remove(&k);
        }
    }

    /// Iterate over entries as (&VarNode, &T).
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            vns_iter: self.vns.iter(),
            data_iter: self.data.iter(),
        }
    }

    /// Iterate over entries as (&mut VarNode, &mut T).
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            vns_iter: self.vns.iter_mut(),
            data_iter: self.data.iter_mut(),
        }
    }

    /// Iterate over entries as (VarNode, &T).
    pub fn items(&self) -> Iter<'_, T> {
        self.iter()
    }
}

impl<T> Default for VarNodeMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// An iterator over the entries of a `VarNodeMap`.
///
/// This struct is created by the `iter` method on `VarNodeMap`.
pub struct Iter<'a, T> {
    vns_iter: std::slice::Iter<'a, VnWrapper>,
    data_iter: std::slice::Iter<'a, T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (&'a VarNode, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.vns_iter.next(), self.data_iter.next()) {
            (Some(wrapper), Some(data)) => Some((&wrapper.0, data)),
            _ => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.data_iter.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.data_iter.len()
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.vns_iter.next_back(), self.data_iter.next_back()) {
            (Some(wrapper), Some(data)) => Some((&wrapper.0, data)),
            _ => None,
        }
    }
}

/// A mutable iterator over the entries of a `VarNodeMap`.
///
/// This struct is created by the `iter_mut` method on `VarNodeMap`.
pub struct IterMut<'a, T> {
    vns_iter: std::slice::IterMut<'a, VnWrapper>,
    data_iter: std::slice::IterMut<'a, T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (&'a mut VarNode, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        match (self.vns_iter.next(), self.data_iter.next()) {
            (Some(wrapper), Some(data)) => Some((&mut wrapper.0, data)),
            _ => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.data_iter.size_hint()
    }
}

impl<'a, T> ExactSizeIterator for IterMut<'a, T> {
    fn len(&self) -> usize {
        self.data_iter.len()
    }
}

impl<'a, T> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match (self.vns_iter.next_back(), self.data_iter.next_back()) {
            (Some(wrapper), Some(data)) => Some((&mut wrapper.0, data)),
            _ => None,
        }
    }
}

/// An owning iterator over the entries of a `VarNodeMap`.
///
/// This struct is created by the `into_iter` method on `VarNodeMap`.
pub struct IntoIter<T> {
    inner: std::vec::IntoIter<(VarNode, T)>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = (VarNode, T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<T> IntoIterator for VarNodeMap<T> {
    type Item = (VarNode, T);
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        // Consume internal vectors, pair keys and values into owned tuples
        let items: Vec<(VarNode, T)> = self.vns.into_iter().map(|w| w.0).zip(self.data).collect();
        IntoIter {
            inner: items.into_iter(),
        }
    }
}

impl<'a, T> IntoIterator for &'a VarNodeMap<T> {
    type Item = (&'a VarNode, &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut VarNodeMap<T> {
    type Item = (&'a mut VarNode, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::VarNodeMap;
    use jingle_sleigh::VarNode;

    #[test]
    fn test_insert_get_contains() {
        let mut m = VarNodeMap::new();

        let vn1 = VarNode {
            space_index: 1,
            offset: 0x10,
            size: 4,
        };
        let vn2 = VarNode {
            space_index: 1,
            offset: 0x08,
            size: 4,
        };

        assert!(!m.contains(&vn1));
        assert!(m.insert(vn1.clone(), 100).is_none());
        assert!(m.contains(&vn1));
        assert_eq!(m.get(&vn1), Some(&100));

        // Insert second in a position that should sort before vn1
        assert!(m.insert(vn2.clone(), 200).is_none());
        assert!(m.contains(&vn2));
        assert_eq!(m.get(&vn2), Some(&200));
        assert_eq!(m.get(&vn1), Some(&100));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_replace_returns_old() {
        let mut m = VarNodeMap::new();
        let vn = VarNode {
            space_index: 0,
            offset: 0x0,
            size: 8,
        };
        assert!(m.insert(vn.clone(), 1).is_none());
        let old = m.insert(vn.clone(), 2);
        assert_eq!(old, Some(1));
        assert_eq!(m.get(&vn), Some(&2));
    }

    #[test]
    fn test_remove_and_indices() {
        let mut m = VarNodeMap::new();
        let a = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        let b = VarNode {
            space_index: 0,
            offset: 4,
            size: 4,
        };
        let c = VarNode {
            space_index: 0,
            offset: 8,
            size: 4,
        };

        m.insert(b.clone(), "b");
        m.insert(a.clone(), "a");
        m.insert(c.clone(), "c");

        // ensure all present
        assert_eq!(m.get(&a), Some(&"a"));
        assert_eq!(m.get(&b), Some(&"b"));
        assert_eq!(m.get(&c), Some(&"c"));

        // remove middle element (originally b)
        let removed = m.remove(&b);
        assert_eq!(removed, Some("b"));
        assert!(!m.contains(&b));
        assert_eq!(m.len(), 2);

        // remaining entries still retrievable
        assert_eq!(m.get(&a), Some(&"a"));
        assert_eq!(m.get(&c), Some(&"c"));
    }

    #[test]
    fn test_iteration_order_is_sorted() {
        let mut m = VarNodeMap::new();
        let v1 = VarNode {
            space_index: 1,
            offset: 0x20,
            size: 4,
        };
        let v2 = VarNode {
            space_index: 0,
            offset: 0x10,
            size: 4,
        };
        let v3 = VarNode {
            space_index: 1,
            offset: 0x10,
            size: 4,
        };

        m.insert(v1.clone(), 1);
        m.insert(v2.clone(), 2);
        m.insert(v3.clone(), 3);

        // iteration should yield keys in sorted order defined by (space_index, offset, size)
        let keys: Vec<(usize, u64, usize)> = m
            .items()
            .map(|(k, _)| (k.space_index, k.offset, k.size))
            .collect();

        assert_eq!(
            keys,
            vec![
                (0, 0x10, 4), // v2
                (1, 0x10, 4), // v3
                (1, 0x20, 4)  // v1
            ]
        );
    }

    #[test]
    fn test_iter_mut() {
        let mut m = VarNodeMap::new();
        let v1 = VarNode {
            space_index: 0,
            offset: 0x10,
            size: 4,
        };
        let v2 = VarNode {
            space_index: 0,
            offset: 0x20,
            size: 4,
        };

        m.insert(v1.clone(), 100);
        m.insert(v2.clone(), 200);

        // Mutate all values using iter_mut
        for (_, value) in m.iter_mut() {
            *value *= 2;
        }

        assert_eq!(m.get(&v1), Some(&200));
        assert_eq!(m.get(&v2), Some(&400));
    }

    #[test]
    fn test_into_iter_consumes() {
        let mut m = VarNodeMap::new();
        let v1 = VarNode {
            space_index: 0,
            offset: 0x10,
            size: 4,
        };
        let v2 = VarNode {
            space_index: 0,
            offset: 0x20,
            size: 4,
        };

        m.insert(v1.clone(), "foo");
        m.insert(v2.clone(), "bar");

        let items: Vec<(VarNode, &str)> = m.into_iter().collect();
        assert_eq!(items.len(), 2);
        // Map was consumed, can't use it anymore
    }
}
