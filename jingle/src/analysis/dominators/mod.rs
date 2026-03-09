use crate::analysis::cfg::{CfgNode, PcodeCfg};
use petgraph::Direction;
use petgraph::algo::dominators::simple_fast;
use petgraph::graph::NodeIndex;
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};

/// Immediate-dominator tree for a `PcodeCfg`.
///
/// `idom[n] = Some(d)` means `d` is the unique immediate dominator of `n`.
/// `idom[n] = None` means `n` is an entry node.
/// Unreachable nodes are absent from the map.
pub struct DominatorTree<N: CfgNode> {
    idom: HashMap<N, Option<N>>,
}

/// Post-dominator tree for a `PcodeCfg` (computed on the reversed CFG).
///
/// `idom[n] = Some(d)` means `d` is the unique immediate post-dominator of `n`.
/// `idom[n] = None` means `n` is an exit (leaf) node.
/// Unreachable nodes are absent from the map.
pub struct PostDominatorTree<N: CfgNode> {
    idom: HashMap<N, Option<N>>,
}

/// Build the idom map for a CFG, optionally reversing all edges (for post-dominators).
///
/// `reversed = false` → dominator tree (entry nodes get `None`).
/// `reversed = true`  → post-dominator tree (leaf nodes get `None`).
fn compute_idom<N: CfgNode, D>(cfg: &PcodeCfg<N, D>, reversed: bool) -> HashMap<N, Option<N>> {
    let g = cfg.graph();

    // Build a scratch StableDiGraph<(), ()> alongside bidirectional index maps.
    let mut scratch: StableDiGraph<(), ()> = StableDiGraph::new();
    let mut node_to_scratch: HashMap<N, NodeIndex> = HashMap::new();
    let mut scratch_to_node: HashMap<NodeIndex, N> = HashMap::new();

    for node in cfg.nodes() {
        let idx = scratch.add_node(());
        node_to_scratch.insert(node.clone(), idx);
        scratch_to_node.insert(idx, node.clone());
    }

    // Mirror edges, reversing direction for post-dominator computation.
    for edge_idx in g.edge_indices() {
        let (src_idx, tgt_idx) = g
            .edge_endpoints(edge_idx)
            .expect("edge index must be valid");
        let src = g.node_weight(src_idx).expect("src must be a valid node");
        let tgt = g.node_weight(tgt_idx).expect("tgt must be a valid node");
        let from = node_to_scratch[src];
        let to = node_to_scratch[tgt];
        if reversed {
            scratch.add_edge(to, from, ());
        } else {
            scratch.add_edge(from, to, ());
        }
    }

    // Collect the "entry" nodes for this direction by checking in-degree on the scratch graph.
    // For forward: nodes with no incoming edges in the original graph.
    // For reversed: nodes with no outgoing edges in the original graph (= leaf nodes).
    let entry_direction = if reversed {
        Direction::Outgoing
    } else {
        Direction::Incoming
    };
    let real_entries: Vec<N> = g
        .externals(entry_direction)
        .map(|idx| g.node_weight(idx).expect("external node must be valid").clone())
        .collect();

    if real_entries.is_empty() {
        return HashMap::new();
    }

    // For multi-entry CFGs add a virtual root with edges to all real entries.
    let virtual_root: Option<NodeIndex>;
    let root_idx: NodeIndex;

    if real_entries.len() == 1 {
        root_idx = node_to_scratch[&real_entries[0]];
        virtual_root = None;
    } else {
        let vroot = scratch.add_node(());
        for entry in &real_entries {
            scratch.add_edge(vroot, node_to_scratch[entry], ());
        }
        root_idx = vroot;
        virtual_root = Some(vroot);
    }

    let dominators = simple_fast(&scratch, root_idx);

    // Translate petgraph NodeIndex results back to N values.
    let mut idom: HashMap<N, Option<N>> = HashMap::new();

    for node in cfg.nodes() {
        let scratch_idx = node_to_scratch[node];

        // Real entry nodes always get idom = None.
        if real_entries.contains(node) {
            idom.insert(node.clone(), None);
            continue;
        }

        match dominators.immediate_dominator(scratch_idx) {
            Some(dom_idx) if Some(dom_idx) == virtual_root => {
                // Direct child of virtual root is effectively an entry node.
                idom.insert(node.clone(), None);
            }
            Some(dom_idx) => {
                let dom_node = scratch_to_node[&dom_idx].clone();
                idom.insert(node.clone(), Some(dom_node));
            }
            None => {
                // Unreachable from root — omit from the map.
            }
        }
    }

    idom
}

impl<N: CfgNode> DominatorTree<N> {
    /// Compute the dominator tree for `cfg`.
    pub fn compute<D>(cfg: &PcodeCfg<N, D>) -> Self {
        Self {
            idom: compute_idom(cfg, false),
        }
    }

    /// Return the immediate dominator of `node`, or `None` for entry nodes.
    /// Returns `None` for unreachable nodes (not present in the tree).
    pub fn immediate_dominator(&self, node: &N) -> Option<&N> {
        self.idom.get(node)?.as_ref()
    }

    /// Iterate over all dominators of `node` (inclusive), walking up the idom chain.
    pub fn dominators<'a>(&'a self, node: &'a N) -> impl Iterator<Item = &'a N> {
        DominatorIter {
            tree: &self.idom,
            current: self.idom.contains_key(node).then_some(node),
        }
    }

    /// Returns `true` if `a` dominates `b` (reflexive: a node dominates itself).
    pub fn dominates(&self, a: &N, b: &N) -> bool {
        self.dominators(b).any(|d| d == a)
    }

    /// Returns `true` if `a` strictly dominates `b` (`a` dominates `b` and `a != b`).
    pub fn strict_dominates(&self, a: &N, b: &N) -> bool {
        a != b && self.dominates(a, b)
    }

    /// Compute the dominance frontier of `node`.
    ///
    /// DF(n) = { y | ∃ predecessor p of y such that n dominates p, and n does not strictly
    ///          dominate y }.
    pub fn dominance_frontier<D>(&self, node: &N, cfg: &PcodeCfg<N, D>) -> HashSet<N> {
        let mut frontier = HashSet::new();
        for y in cfg.nodes() {
            if self.strict_dominates(node, y) {
                continue;
            }
            if let Some(&y_idx) = cfg.indices.get(y) {
                let has_dominated_pred = cfg
                    .graph()
                    .edges_directed(y_idx, Direction::Incoming)
                    .any(|e| {
                        cfg.graph()
                            .node_weight(e.source())
                            .is_some_and(|pred| self.dominates(node, pred))
                    });
                if has_dominated_pred {
                    frontier.insert(y.clone());
                }
            }
        }
        frontier
    }
}

impl<N: CfgNode> PostDominatorTree<N> {
    /// Compute the post-dominator tree for `cfg`.
    pub fn compute<D>(cfg: &PcodeCfg<N, D>) -> Self {
        Self {
            idom: compute_idom(cfg, true),
        }
    }

    /// Return the immediate post-dominator of `node`, or `None` for exit nodes.
    pub fn immediate_dominator(&self, node: &N) -> Option<&N> {
        self.idom.get(node)?.as_ref()
    }

    /// Iterate over all post-dominators of `node` (inclusive).
    pub fn dominators<'a>(&'a self, node: &'a N) -> impl Iterator<Item = &'a N> {
        DominatorIter {
            tree: &self.idom,
            current: self.idom.contains_key(node).then_some(node),
        }
    }

    /// Returns `true` if `a` post-dominates `b`.
    pub fn dominates(&self, a: &N, b: &N) -> bool {
        self.dominators(b).any(|d| d == a)
    }

    /// Returns `true` if `a` strictly post-dominates `b`.
    pub fn strict_dominates(&self, a: &N, b: &N) -> bool {
        a != b && self.dominates(a, b)
    }
}

/// Iterator that walks up the idom chain from a starting node (inclusive).
struct DominatorIter<'a, N: CfgNode> {
    tree: &'a HashMap<N, Option<N>>,
    current: Option<&'a N>,
}

impl<'a, N: CfgNode> Iterator for DominatorIter<'a, N> {
    type Item = &'a N;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.current?;
        self.current = match self.tree.get(node) {
            Some(Some(parent)) => Some(parent),
            _ => None,
        };
        Some(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::cfg::PcodeCfg;
    use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

    /// Shorthand: node with machine address `m` and pcode offset 0.
    fn n(m: u64) -> ConcretePcodeAddress {
        ConcretePcodeAddress::new(m, 0)
    }

    /// Build a `PcodeCfg<ConcretePcodeAddress, ()>` from a list of (from, to) edges.
    fn make_cfg(edges: &[(u64, u64)]) -> PcodeCfg<ConcretePcodeAddress, ()> {
        let mut cfg = PcodeCfg::new();
        for &(from, to) in edges {
            cfg.add_edge(n(from), n(to), ());
        }
        cfg
    }

    // ── Test 1: Diamond CFG  A→B, A→C, B→D, C→D ────────────────────────────

    #[test]
    fn diamond_idom() {
        let cfg = make_cfg(&[(0, 1), (0, 2), (1, 3), (2, 3)]);
        let dt = DominatorTree::compute(&cfg);

        assert_eq!(dt.immediate_dominator(&n(1)), Some(&n(0)));
        assert_eq!(dt.immediate_dominator(&n(2)), Some(&n(0)));
        assert_eq!(dt.immediate_dominator(&n(3)), Some(&n(0)));
        assert_eq!(dt.immediate_dominator(&n(0)), None);
    }

    #[test]
    fn diamond_dominates() {
        let cfg = make_cfg(&[(0, 1), (0, 2), (1, 3), (2, 3)]);
        let dt = DominatorTree::compute(&cfg);

        assert!(dt.dominates(&n(0), &n(3)));
        assert!(!dt.dominates(&n(1), &n(3)));
        assert!(!dt.dominates(&n(2), &n(3)));
        // Reflexive
        assert!(dt.dominates(&n(0), &n(0)));
    }

    #[test]
    fn diamond_dominance_frontier() {
        let cfg = make_cfg(&[(0, 1), (0, 2), (1, 3), (2, 3)]);
        let dt = DominatorTree::compute(&cfg);

        let df_b = dt.dominance_frontier(&n(1), &cfg);
        assert_eq!(df_b, HashSet::from([n(3)]));

        let df_c = dt.dominance_frontier(&n(2), &cfg);
        assert_eq!(df_c, HashSet::from([n(3)]));
    }

    #[test]
    fn diamond_post_dominator() {
        let cfg = make_cfg(&[(0, 1), (0, 2), (1, 3), (2, 3)]);
        let pdt = PostDominatorTree::compute(&cfg);

        assert_eq!(pdt.immediate_dominator(&n(3)), None);
        assert_eq!(pdt.immediate_dominator(&n(1)), Some(&n(3)));
        assert_eq!(pdt.immediate_dominator(&n(2)), Some(&n(3)));
        assert_eq!(pdt.immediate_dominator(&n(0)), Some(&n(3)));

        assert!(pdt.dominates(&n(3), &n(0)));
    }

    // ── Test 2: Linear chain  A→B→C ─────────────────────────────────────────

    #[test]
    fn linear_chain_idom() {
        let cfg = make_cfg(&[(0, 1), (1, 2)]);
        let dt = DominatorTree::compute(&cfg);

        assert_eq!(dt.immediate_dominator(&n(0)), None);
        assert_eq!(dt.immediate_dominator(&n(1)), Some(&n(0)));
        assert_eq!(dt.immediate_dominator(&n(2)), Some(&n(1)));
    }

    #[test]
    fn linear_chain_post_idom() {
        let cfg = make_cfg(&[(0, 1), (1, 2)]);
        let pdt = PostDominatorTree::compute(&cfg);

        assert_eq!(pdt.immediate_dominator(&n(2)), None);
        assert_eq!(pdt.immediate_dominator(&n(1)), Some(&n(2)));
        assert_eq!(pdt.immediate_dominator(&n(0)), Some(&n(1)));
    }

    // ── Test 3: Loop  A→B→C→B, C→D ─────────────────────────────────────────

    #[test]
    fn loop_idom() {
        // A→B, B→C, C→B (back-edge), C→D
        let cfg = make_cfg(&[(0, 1), (1, 2), (2, 1), (2, 3)]);
        let dt = DominatorTree::compute(&cfg);

        assert_eq!(dt.immediate_dominator(&n(0)), None);
        assert_eq!(dt.immediate_dominator(&n(1)), Some(&n(0)));
        assert_eq!(dt.immediate_dominator(&n(2)), Some(&n(1)));
        assert_eq!(dt.immediate_dominator(&n(3)), Some(&n(2)));
    }

    #[test]
    fn loop_b_post_dominates_a() {
        // A→B, B→C, C→B (back-edge), C→D
        let cfg = make_cfg(&[(0, 1), (1, 2), (2, 1), (2, 3)]);
        let pdt = PostDominatorTree::compute(&cfg);
        // B is on every path from A to D, so B post-dominates A.
        assert!(pdt.dominates(&n(1), &n(0)));
    }

    // ── Test 4: Multi-entry ──────────────────────────────────────────────────

    #[test]
    fn multi_entry_both_get_none_idom() {
        let mut cfg: PcodeCfg<ConcretePcodeAddress, ()> = PcodeCfg::new();
        cfg.add_node(n(0));
        cfg.add_node(n(1));
        let dt = DominatorTree::compute(&cfg);

        assert_eq!(dt.immediate_dominator(&n(0)), None);
        assert_eq!(dt.immediate_dominator(&n(1)), None);
    }

    // ── Test 5: Multi-exit ───────────────────────────────────────────────────

    #[test]
    fn multi_exit_both_get_none_post_idom() {
        // A forks to B and C, neither rejoins.
        let cfg = make_cfg(&[(0, 1), (0, 2)]);
        let pdt = PostDominatorTree::compute(&cfg);

        assert_eq!(pdt.immediate_dominator(&n(1)), None);
        assert_eq!(pdt.immediate_dominator(&n(2)), None);
    }
}
