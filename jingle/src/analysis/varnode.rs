use jingle_sleigh::VarNode;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct VarNodeSpaceSet {
    vn_starts: BTreeMap<u64, u64>,
    vn_ends: BTreeMap<u64, u64>,
}

impl VarNodeSpaceSet {
    pub fn insert(&mut self, vn: Range<u64>) {
        let o: Vec<_> = self.get_overlaps(&vn).collect();
        // clear all overlaps
        o.iter().for_each(|range| {
            self.vn_starts.remove(&range.start);
            self.vn_ends.remove(&range.end);
        });
        // get_min
        let vn_min = vn.start;
        let vn_max = vn.end;
        let min = vn_min.min(o.first().map(|range| range.start).unwrap_or(vn_min));
        let max = vn_max.max(o.last().map(|range| range.end).unwrap_or(vn_max));
        self.vn_starts.insert(min, max);
        self.vn_ends.insert(max, min);
    }

    pub fn intersect(&mut self, other: &Self) {
        let mut n: VarNodeSpaceSet = Default::default();
        other
            .vn_starts
            .iter()
            .map(|a| (*a.0..*a.1))
            .for_each(|other_range| {
                for x in self.get_overlaps(&other_range) {
                    let start = x.start.max(other_range.start);
                    let end = x.end.min(other_range.end);
                    n.insert(start..end);
                }
            });
        *self = n;
    }

    pub fn covers(&self, range: &Range<u64>) -> bool {
        let overlaps: Vec<_> = self.get_overlaps(range).collect();
        if overlaps.is_empty() {
            return false;
        }
        let mut cur = range.start;
        for overlap in &overlaps {
            if overlap.start > cur {
                return false; // there's a gap
            }
            cur = overlap.end;
        }
        if cur < range.end {
            return false; // there's a gap at the end
        }
        true
    }

    pub fn union(&mut self, other: &Self) {
        for (start, end) in &other.vn_starts {
            self.insert(*start..*end);
        }
    }

    pub fn subtract(&mut self, other: &Self) {
        for (start, end) in &other.vn_starts {
            let range = *start..*end;
            let overlaps: Vec<_> = self.get_overlaps(&range).collect();
            for overlap in overlaps {
                self.vn_starts.remove(&overlap.start);
                self.vn_ends.remove(&overlap.end);
                if overlap.start < range.start {
                    self.insert(overlap.start..range.start);
                }
                if overlap.end > range.end {
                    self.insert(range.end..overlap.end);
                }
            }
        }
    }

    pub fn ranges(&self) -> impl Iterator<Item = Range<u64>> {
        self.vn_starts.iter().map(|(start, end)| *start..*end)
    }
    fn get_overlaps(&self, vn: &Range<u64>) -> impl Iterator<Item = Range<u64>> {
        let first = self
            .vn_ends
            .range(vn.start..)
            .next()
            .filter(|&(_end, &start)| start <= vn.end)
            .map(|(_a, b)| *b)
            .unwrap_or(vn.start);

        let last = self
            .vn_starts
            .range(..vn.end)
            .next_back()
            .filter(|&(_start, &end)| end >= vn.start)
            .map(|(_a, b)| *b)
            .unwrap_or(vn.end);
        self.vn_starts.range(first..last).map(|(a, b)| (*a..*b))
    }
}

impl PartialOrd for VarNodeSpaceSet {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let all_covered = other.ranges().all(|range| self.covers(&range));
        let other_covered = self.ranges().all(|range| other.covers(&range));
        if all_covered && other_covered {
            Some(Ordering::Equal)
        } else if all_covered {
            Some(Ordering::Less)
        } else if other_covered {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct VarNodeSet {
    space_map: HashMap<usize, VarNodeSpaceSet>,
}

impl VarNodeSet {
    pub fn insert(&mut self, vn: &VarNode) {
        self.get_map_mut(vn.space_index).insert(vn.into())
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let mut n = self.clone();
        for (space, map) in &mut n.space_map {
            if let Some(other_map) = other.space_map.get(space) {
                map.intersect(other_map);
            }
        }
        n
    }

    pub fn union(&mut self, other: &Self) {
        for (space, map) in &other.space_map {
            self.get_map_mut(*space).union(map);
        }
    }

    pub fn subtract(&mut self, other: &Self) {
        for (space, map) in &other.space_map {
            if let Some(our_map) = self.space_map.get_mut(space) {
                our_map.subtract(map);
            }
        }
    }

    pub fn covers(&self, vn: &VarNode) -> bool {
        if let Some(map) = self.space_map.get(&vn.space_index) {
            map.covers(&vn.into())
        } else {
            false
        }
    }

    pub fn varnodes(&self) -> impl Iterator<Item = VarNode> {
        self.space_map.iter().flat_map(|(space, map)| {
            map.ranges().map(|range| VarNode {
                space_index: *space,
                offset: range.start,
                size: (range.end - range.start) as usize,
            })
        })
    }

    fn get_map_mut(&mut self, s: usize) -> &mut VarNodeSpaceSet {
        if let std::collections::hash_map::Entry::Vacant(e) = self.space_map.entry(s) {
            let n = Default::default();
            e.insert(n);
            self.space_map.get_mut(&s).unwrap()
        } else {
            self.space_map.get_mut(&s).unwrap()
        }
    }
}

impl PartialOrd for VarNodeSet {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let keys = self.space_map.keys();
        let other_keys = other.space_map.keys();
        let keys: HashSet<_> = keys.chain(other_keys).collect();
        let t1 = Default::default();
        let t2 = Default::default();
        let mut last: Option<Ordering> = Some(Ordering::Equal);
        for space_idx in keys {
            let our_space = self.space_map.get(space_idx).unwrap_or(&t1);
            let their = other.space_map.get(space_idx).unwrap_or(&t2);
            let s = our_space.partial_cmp(their);
            if s.is_none() {
                return None; // not comparable
            } else {
                let s = s.unwrap();
                if let Some(last_val) = last {
                    match (last_val, s) {
                        (Ordering::Equal, a) => {
                            last = Some(a);
                        }
                        (last_val, this) => {
                            if last_val == Ordering::Equal {
                                last = Some(this);
                            } else if last_val != this && this != Ordering::Equal {
                                return None;
                            }
                        }
                    }
                } else {
                    last = Some(s);
                }
            }
        }
        last
    }
}

#[cfg(test)]
mod tests {
    use crate::analysis::varnode::VarNodeSet;
    use crate::sleigh::VarNode;
    use std::cmp::Ordering;

    #[test]
    fn test_single_insert() {
        let mut set = VarNodeSet::default();
        let vn = VarNode {
            space_index: 0,
            offset: 4,
            size: 4,
        };
        set.insert(&vn);
        let items = set.varnodes().collect::<Vec<_>>();
        assert_eq!(items, vec![vn]);
    }

    #[test]
    fn test_covers() {
        let mut set = VarNodeSet::default();
        let vn = VarNode {
            space_index: 0,
            offset: 4,
            size: 4,
        };
        set.insert(&vn);
        assert!(set.covers(&vn));
        assert!(!set.covers(&VarNode {
            space_index: 0,
            offset: 4,
            size: 43,
        }));
    }

    #[test]
    fn test_ord() {
        let mut set = VarNodeSet::default();
        let vn = VarNode {
            space_index: 0,
            offset: 4,
            size: 4,
        };
        set.insert(&vn);
        let mut set2 = VarNodeSet::default();
        let vn2 = VarNode {
            space_index: 0,
            offset: 4,
            size: 1,
        };
        set2.insert(&vn2);
        assert_eq!(set.partial_cmp(&set2), Some(Ordering::Less));
        set2.insert(&vn);
        assert_eq!(set.partial_cmp(&set2), Some(Ordering::Equal));
        set2.insert(&VarNode {
            space_index: 0,
            offset: 80,
            size: 4,
        });
        assert_eq!(set.partial_cmp(&set2), Some(Ordering::Greater));
        set.insert(&VarNode {
            space_index: 1,
            offset: 80,
            size: 4,
        });
        assert_eq!(set.partial_cmp(&set2), None);
    }

    #[test]
    fn test_overlapping_insert() {
        let mut set = VarNodeSet::default();
        let vn = VarNode {
            space_index: 0,
            offset: 4,
            size: 4,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 6,
            size: 4,
        };
        set.insert(&vn);
        set.insert(&vn2);
        let items = set.varnodes().collect::<Vec<_>>();
        assert_eq!(
            items,
            vec![VarNode {
                space_index: 0,
                offset: 4,
                size: 6
            }]
        );
    }

    #[test]
    fn test_nonoverlapping_insert() {
        let mut set = VarNodeSet::default();
        let vn = VarNode {
            space_index: 0,
            offset: 4,
            size: 4,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 9,
            size: 4,
        };
        set.insert(&vn);
        set.insert(&vn2);
        let items = set.varnodes().collect::<Vec<_>>();
        assert_eq!(items, vec![vn, vn2]);
    }

    #[test]
    fn test_intersection_partial() {
        let mut set = VarNodeSet::default();
        let mut set2 = VarNodeSet::default();
        let vn = VarNode {
            space_index: 0,
            offset: 4,
            size: 4,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 9,
            size: 4,
        };
        let vn3 = VarNode {
            space_index: 0,
            offset: 5,
            size: 6,
        };
        set.insert(&vn);
        set.insert(&vn2);
        set2.insert(&vn3);
        let intersect = set.intersect(&set2);
        let intersect2 = set2.intersect(&set);
        assert_eq!(intersect, intersect2);
        let items = intersect.varnodes().collect::<Vec<_>>();
        assert_eq!(
            items,
            vec![
                VarNode {
                    space_index: 0,
                    offset: 5,
                    size: 3
                },
                VarNode {
                    space_index: 0,
                    offset: 9,
                    size: 2
                }
            ]
        );
    }

    #[test]
    fn test_empty_set_behavior() {
        let set = VarNodeSet::default();
        let vn = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        assert!(!set.covers(&vn));
        assert_eq!(set.varnodes().count(), 0);
        let set2 = VarNodeSet::default();
        assert_eq!(set.partial_cmp(&set2), Some(Ordering::Equal));
    }

    #[test]
    fn test_multi_space_insert_and_cmp() {
        let mut set = VarNodeSet::default();
        let mut set2 = VarNodeSet::default();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        let vn2 = VarNode {
            space_index: 1,
            offset: 0,
            size: 4,
        };
        set.insert(&vn1);
        set2.insert(&vn2);
        assert_eq!(set.partial_cmp(&set2), None);
        set.insert(&vn2);
        set2.insert(&vn1);
        assert_eq!(set.partial_cmp(&set2), Some(Ordering::Equal));
    }

    #[test]
    fn test_full_overlap_insert() {
        let mut set = VarNodeSet::default();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0,
            size: 8,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 2,
            size: 2,
        };
        set.insert(&vn1);
        set.insert(&vn2);
        let items = set.varnodes().collect::<Vec<_>>();
        assert_eq!(items, vec![vn1]);
    }

    #[test]
    fn test_adjacent_insert_merging() {
        let mut set = VarNodeSet::default();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 4,
            size: 4,
        };
        set.insert(&vn1);
        set.insert(&vn2);
        let items = set.varnodes().collect::<Vec<_>>();
        assert_eq!(
            items,
            vec![VarNode {
                space_index: 0,
                offset: 0,
                size: 8
            }]
        );
    }

    #[test]
    fn test_covers_partial_and_full() {
        let mut set = VarNodeSet::default();
        let vn = VarNode {
            space_index: 0,
            offset: 0,
            size: 8,
        };
        set.insert(&vn);
        let partial = VarNode {
            space_index: 0,
            offset: 2,
            size: 2,
        };
        let outside = VarNode {
            space_index: 0,
            offset: 8,
            size: 2,
        };
        assert!(set.covers(&vn));
        assert!(set.covers(&partial));
        assert!(!set.covers(&outside));
    }

    #[test]
    fn test_intersect_disjoint() {
        let mut set1 = VarNodeSet::default();
        let mut set2 = VarNodeSet::default();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 8,
            size: 4,
        };
        set1.insert(&vn1);
        set2.insert(&vn2);
        let intersect = set1.intersect(&set2);
        assert_eq!(intersect.varnodes().count(), 0);
    }

    #[test]
    fn test_intersect_full_overlap() {
        let mut set1 = VarNodeSet::default();
        let mut set2 = VarNodeSet::default();
        let vn = VarNode {
            space_index: 0,
            offset: 0,
            size: 8,
        };
        set1.insert(&vn);
        set2.insert(&vn);
        let intersect = set1.intersect(&set2);
        let items = intersect.varnodes().collect::<Vec<_>>();
        assert_eq!(items, vec![vn]);
    }

    #[test]
    fn test_union_basic() {
        let mut set1 = VarNodeSet::default();
        let mut set2 = VarNodeSet::default();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 4,
            size: 4,
        };
        set1.insert(&vn1);
        set2.insert(&vn2);
        set1.union(&set2);
        let items = set1.varnodes().collect::<Vec<_>>();
        assert_eq!(
            items,
            vec![VarNode {
                space_index: 0,
                offset: 0,
                size: 8
            }]
        );
    }

    #[test]
    fn test_union_disjoint_spaces() {
        let mut set1 = VarNodeSet::default();
        let mut set2 = VarNodeSet::default();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        let vn2 = VarNode {
            space_index: 1,
            offset: 0,
            size: 4,
        };
        set1.insert(&vn1);
        set2.insert(&vn2);
        set1.union(&set2);
        let mut items = set1.varnodes().collect::<Vec<_>>();
        items.sort_by_key(|vn| vn.space_index);
        assert_eq!(items, vec![vn1, vn2]);
    }

    #[test]
    fn test_subtract_partial_overlap() {
        let mut set1 = VarNodeSet::default();
        let mut set2 = VarNodeSet::default();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0,
            size: 8,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 2,
            size: 4,
        };
        set1.insert(&vn1);
        set2.insert(&vn2);
        set1.subtract(&set2);
        let items = set1.varnodes().collect::<Vec<_>>();
        assert_eq!(
            items,
            vec![
                VarNode {
                    space_index: 0,
                    offset: 0,
                    size: 2
                },
                VarNode {
                    space_index: 0,
                    offset: 6,
                    size: 2
                }
            ]
        );
    }

    #[test]
    fn test_subtract_full_overlap() {
        let mut set1 = VarNodeSet::default();
        let mut set2 = VarNodeSet::default();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        set1.insert(&vn1);
        set2.insert(&vn2);
        set1.subtract(&set2);
        assert_eq!(set1.varnodes().count(), 0);
    }

    #[test]
    fn test_subtract_disjoint() {
        let mut set1 = VarNodeSet::default();
        let mut set2 = VarNodeSet::default();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 8,
            size: 4,
        };
        set1.insert(&vn1);
        set2.insert(&vn2);
        set1.subtract(&set2);
        let items = set1.varnodes().collect::<Vec<_>>();
        assert_eq!(items, vec![vn1]);
    }

    #[test]
    fn test_covers_empty_set() {
        let set = VarNodeSet::default();
        let vn = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        assert!(!set.covers(&vn));
    }

    #[test]
    fn test_varnodes_iterator_empty() {
        let set = VarNodeSet::default();
        assert_eq!(set.varnodes().count(), 0);
    }

    #[test]
    fn test_insert_and_subtract_adjacent() {
        let mut set = VarNodeSet::default();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 4,
            size: 4,
        };
        set.insert(&vn1);
        set.insert(&vn2);
        let mut set2 = VarNodeSet::default();
        let vn3 = VarNode {
            space_index: 0,
            offset: 2,
            size: 4,
        };
        set2.insert(&vn3);
        set.subtract(&set2);
        let items = set.varnodes().collect::<Vec<_>>();
        assert_eq!(
            items,
            vec![
                VarNode {
                    space_index: 0,
                    offset: 0,
                    size: 2
                },
                VarNode {
                    space_index: 0,
                    offset: 6,
                    size: 2
                }
            ]
        );
    }

    #[test]
    fn test_union_with_empty() {
        let mut set1 = VarNodeSet::default();
        let set2 = VarNodeSet::default();
        let vn = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        set1.insert(&vn);
        set1.union(&set2);
        let items = set1.varnodes().collect::<Vec<_>>();
        assert_eq!(items, vec![vn]);
    }

    #[test]
    fn test_subtract_empty() {
        let mut set1 = VarNodeSet::default();
        let set2 = VarNodeSet::default();
        let vn = VarNode {
            space_index: 0,
            offset: 0,
            size: 4,
        };
        set1.insert(&vn);
        set1.subtract(&set2);
        let items = set1.varnodes().collect::<Vec<_>>();
        assert_eq!(items, vec![vn]);
    }

    #[test]
    fn test_insert_multiple_nonoverlapping() {
        let mut set = VarNodeSet::default();
        let vn1 = VarNode {
            space_index: 0,
            offset: 0,
            size: 2,
        };
        let vn2 = VarNode {
            space_index: 0,
            offset: 4,
            size: 2,
        };
        let vn3 = VarNode {
            space_index: 0,
            offset: 8,
            size: 2,
        };
        set.insert(&vn1);
        set.insert(&vn2);
        set.insert(&vn3);
        let mut items = set.varnodes().collect::<Vec<_>>();
        items.sort_by_key(|vn| vn.offset);
        assert_eq!(items, vec![vn1, vn2, vn3]);
    }
}
