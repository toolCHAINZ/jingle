//! This is a placeholder structuring algorithm, written with agent assistance given existing graph structuring literature,
//! the CPA traits in this repo, and the StructuredCfg type as a goal. It has not been evaluated except on simple
//! test-cases and is subject to change.
use crate::analysis::cfg::{CfgNode, PcodeCfg};
use crate::analysis::dominators::{DominatorTree, PostDominatorTree};
use jingle_sleigh::{JingleDisplay, SleighArchInfo};
use std::collections::HashSet;
use std::fmt;

pub trait CbranchInfo {
    fn cbranch_input0(&self) -> Option<&jingle_sleigh::VarNode>;
}

impl CbranchInfo for () {
    fn cbranch_input0(&self) -> Option<&jingle_sleigh::VarNode> {
        None
    }
}

impl CbranchInfo for jingle_sleigh::PcodeOperation {
    fn cbranch_input0(&self) -> Option<&jingle_sleigh::VarNode> {
        if let jingle_sleigh::PcodeOperation::CBranch { input0, .. } = self {
            Some(input0)
        } else {
            None
        }
    }
}

impl<'a> CbranchInfo for jingle_sleigh::PcodeOpRef<'a> {
    fn cbranch_input0(&self) -> Option<&jingle_sleigh::VarNode> {
        use std::ops::Deref;
        self.deref().cbranch_input0()
    }
}

impl CbranchInfo for Vec<jingle_sleigh::PcodeOperation> {
    fn cbranch_input0(&self) -> Option<&jingle_sleigh::VarNode> {
        self.last()?.cbranch_input0()
    }
}

#[derive(Debug, Clone)]
pub enum StructuredCfg<N> {
    Block(N),
    Sequence(Vec<StructuredCfg<N>>),
    IfElse {
        header: N,
        then_branch: Box<StructuredCfg<N>>,
        else_branch: Box<StructuredCfg<N>>,
    },
    Loop {
        header: N,
        body: Box<StructuredCfg<N>>,
    },
}

fn fmt_indent<N: fmt::Display>(
    cfg: &StructuredCfg<N>,
    f: &mut fmt::Formatter<'_>,
    depth: usize,
) -> fmt::Result {
    let pad = "  ".repeat(depth);
    match cfg {
        StructuredCfg::Block(n) => writeln!(f, "{pad}block({n})"),
        StructuredCfg::Sequence(items) => {
            for item in items {
                fmt_indent(item, f, depth)?;
            }
            Ok(())
        }
        StructuredCfg::IfElse {
            header,
            then_branch,
            else_branch,
        } => {
            writeln!(f, "{pad}if {header} {{")?;
            fmt_indent(then_branch, f, depth + 1)?;
            writeln!(f, "{pad}}} else {{")?;
            fmt_indent(else_branch, f, depth + 1)?;
            writeln!(f, "{pad}}}")
        }
        StructuredCfg::Loop { header, body } => {
            writeln!(f, "{pad}loop {header} {{")?;
            fmt_indent(body, f, depth + 1)?;
            writeln!(f, "{pad}}}")
        }
    }
}

impl<N: fmt::Display> fmt::Display for StructuredCfg<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_indent(self, f, 0)
    }
}

fn fmt_jingle_indent<N: JingleDisplay>(
    cfg: &StructuredCfg<N>,
    f: &mut fmt::Formatter<'_>,
    ctx: &SleighArchInfo,
    depth: usize,
) -> fmt::Result {
    let pad = "  ".repeat(depth);
    match cfg {
        StructuredCfg::Block(n) => writeln!(f, "{pad}block({})", n.display(ctx)),
        StructuredCfg::Sequence(items) => {
            for item in items {
                fmt_jingle_indent(item, f, ctx, depth)?;
            }
            Ok(())
        }
        StructuredCfg::IfElse {
            header,
            then_branch,
            else_branch,
        } => {
            writeln!(f, "{pad}if {} {{", header.display(ctx))?;
            fmt_jingle_indent(then_branch, f, ctx, depth + 1)?;
            writeln!(f, "{pad}}} else {{")?;
            fmt_jingle_indent(else_branch, f, ctx, depth + 1)?;
            writeln!(f, "{pad}}}")
        }
        StructuredCfg::Loop { header, body } => {
            writeln!(f, "{pad}loop {} {{", header.display(ctx))?;
            fmt_jingle_indent(body, f, ctx, depth + 1)?;
            writeln!(f, "{pad}}}")
        }
    }
}

impl<N: JingleDisplay> JingleDisplay for StructuredCfg<N> {
    fn fmt_jingle(&self, f: &mut fmt::Formatter<'_>, ctx: &SleighArchInfo) -> fmt::Result {
        fmt_jingle_indent(self, f, ctx, 0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StructuringError {
    #[error("irreducible control flow")]
    Irreducible,
    #[error("empty CFG")]
    Empty,
}

pub fn structure<N: CfgNode, D: Clone + CbranchInfo>(
    mut cfg: PcodeCfg<N, D>,
) -> Result<StructuredCfg<N>, StructuringError> {
    cfg.trim_symbolic_leaves();
    let entries: Vec<&N> = cfg.entry_nodes().collect();
    match entries.len() {
        0 => return Err(StructuringError::Empty),
        1 => {}
        _ => return Err(StructuringError::Irreducible),
    }
    let entry = entries[0].clone();
    let dom = cfg.dominator_tree();
    let pdom = cfg.post_dominator_tree();
    check_reducibility(&cfg)?;
    structure_from(&entry, None, &cfg, &dom, &pdom, &HashSet::new())
}

/// Computes all strongly connected components using Kosaraju's algorithm.
fn compute_sccs<N: CfgNode, D: Clone>(cfg: &PcodeCfg<N, D>) -> Vec<HashSet<N>> {
    let all_nodes: Vec<N> = cfg.nodes().cloned().collect();

    // Phase 1: forward DFS, record post-order finish times.
    let mut visited: HashSet<N> = HashSet::new();
    let mut post_order: Vec<N> = Vec::new();

    for start in &all_nodes {
        if visited.contains(start) {
            continue;
        }
        visited.insert(start.clone());
        let start_succs: Vec<N> = cfg
            .successors(start)
            .unwrap_or_default()
            .into_iter()
            .cloned()
            .collect();
        let mut dfs_stack: Vec<(N, usize, Vec<N>)> = vec![(start.clone(), 0, start_succs)];
        loop {
            let next = {
                let Some(frame) = dfs_stack.last_mut() else {
                    break;
                };
                if frame.1 < frame.2.len() {
                    let succ = frame.2[frame.1].clone();
                    frame.1 += 1;
                    Some(succ)
                } else {
                    None
                }
            };
            match next {
                Some(succ) => {
                    if visited.insert(succ.clone()) {
                        let succs: Vec<N> = cfg
                            .successors(&succ)
                            .unwrap_or_default()
                            .into_iter()
                            .cloned()
                            .collect();
                        dfs_stack.push((succ, 0, succs));
                    }
                }
                None => {
                    let Some((node, _, _)) = dfs_stack.pop() else {
                        break;
                    };
                    post_order.push(node);
                }
            }
        }
    }

    // Phase 2: reverse DFS in reverse post-order assigns each node to an SCC.
    let mut visited2: HashSet<N> = HashSet::new();
    let mut sccs: Vec<HashSet<N>> = Vec::new();

    for start in post_order.iter().rev() {
        if visited2.contains(start) {
            continue;
        }
        let mut scc: HashSet<N> = HashSet::new();
        let mut stack: Vec<N> = vec![start.clone()];
        while let Some(node) = stack.pop() {
            if !visited2.insert(node.clone()) {
                continue;
            }
            scc.insert(node.clone());
            for pred in cfg.predecessors(&node).unwrap_or_default() {
                if !visited2.contains(pred) {
                    stack.push(pred.clone());
                }
            }
        }
        sccs.push(scc);
    }

    sccs
}

fn check_reducibility<N: CfgNode, D: Clone>(cfg: &PcodeCfg<N, D>) -> Result<(), StructuringError> {
    // A CFG is irreducible iff any strongly connected component (SCC) with more
    // than one node has multiple external entry points — nodes inside the SCC
    // that have at least one predecessor outside the SCC.  Reducible natural
    // loops always have exactly one such header.
    for scc in compute_sccs(cfg) {
        if scc.len() <= 1 {
            continue;
        }
        let external_entries = scc
            .iter()
            .filter(|node| {
                cfg.predecessors(*node)
                    .unwrap_or_default()
                    .into_iter()
                    .any(|pred| !scc.contains(pred))
            })
            .count();
        if external_entries > 1 {
            return Err(StructuringError::Irreducible);
        }
    }
    Ok(())
}

fn natural_loop_body<N: CfgNode, D: Clone>(
    header: &N,
    latches: &[N],
    cfg: &PcodeCfg<N, D>,
    dom: &DominatorTree<N>,
) -> HashSet<N> {
    let mut body: HashSet<N> = HashSet::new();
    body.insert(header.clone());
    let mut worklist: Vec<N> = latches.to_vec();
    while let Some(node) = worklist.pop() {
        if body.contains(&node) {
            continue;
        }
        body.insert(node.clone());
        for pred in cfg.predecessors(&node).unwrap_or_default() {
            if !body.contains(pred) && dom.dominates(header, pred) {
                worklist.push(pred.clone());
            }
        }
    }
    body
}

fn flatten<N: CfgNode>(items: Vec<StructuredCfg<N>>) -> StructuredCfg<N> {
    let mut result: Vec<StructuredCfg<N>> = Vec::new();
    for item in items {
        match item {
            StructuredCfg::Sequence(inner) => result.extend(inner),
            other => result.push(other),
        }
    }
    match result.len() {
        0 => StructuredCfg::Sequence(vec![]),
        1 => result.remove(0),
        _ => StructuredCfg::Sequence(result),
    }
}

fn structure_from<N: CfgNode, D: Clone + CbranchInfo>(
    current: &N,
    stop_at: Option<&N>,
    cfg: &PcodeCfg<N, D>,
    dom: &DominatorTree<N>,
    pdom: &PostDominatorTree<N>,
    back_edges: &HashSet<(N, N)>,
) -> Result<StructuredCfg<N>, StructuringError> {
    if stop_at == Some(current) {
        return Ok(StructuredCfg::Sequence(vec![]));
    }

    let fwd_succs: Vec<N> = cfg
        .successors(current)
        .unwrap_or_default()
        .into_iter()
        .filter(|s| !back_edges.contains(&(current.clone(), (*s).clone())))
        .cloned()
        .collect();

    let latches: Vec<N> = cfg
        .predecessors(current)
        .unwrap_or_default()
        .into_iter()
        .filter(|p| dom.dominates(current, p))
        .cloned()
        .collect();

    if !latches.is_empty() {
        return structure_loop(current, stop_at, cfg, dom, pdom, back_edges, &latches);
    }

    match fwd_succs.len() {
        0 => Ok(StructuredCfg::Block(current.clone())),
        1 => {
            let rest = structure_from(&fwd_succs[0], stop_at, cfg, dom, pdom, back_edges)?;
            Ok(flatten(vec![StructuredCfg::Block(current.clone()), rest]))
        }
        2 => {
            let merge = pdom.immediate_dominator(current);
            let taken_idx = cfg
                .get_op_at(current)
                .and_then(|op| op.cbranch_input0())
                .and_then(|input0| {
                    current.concrete_location().and_then(|addr| {
                        let target =
                            crate::modeling::machine::cpu::concrete::ConcretePcodeAddress
                                ::resolve_from_varnode(input0, addr);
                        fwd_succs
                            .iter()
                            .position(|s| s.concrete_location() == Some(target))
                    })
                })
                .unwrap_or(0);
            let else_idx = 1 - taken_idx;
            let then_branch =
                structure_from(&fwd_succs[taken_idx], merge, cfg, dom, pdom, back_edges)?;
            let else_branch =
                structure_from(&fwd_succs[else_idx], merge, cfg, dom, pdom, back_edges)?;
            let rest = match merge {
                Some(m) => structure_from(m, stop_at, cfg, dom, pdom, back_edges)?,
                None => StructuredCfg::Sequence(vec![]),
            };
            Ok(flatten(vec![
                StructuredCfg::IfElse {
                    header: current.clone(),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                },
                rest,
            ]))
        }
        _ => Err(StructuringError::Irreducible),
    }
}

fn structure_loop<N: CfgNode, D: Clone + CbranchInfo>(
    header: &N,
    stop_at: Option<&N>,
    cfg: &PcodeCfg<N, D>,
    dom: &DominatorTree<N>,
    pdom: &PostDominatorTree<N>,
    outer_back_edges: &HashSet<(N, N)>,
    latches: &[N],
) -> Result<StructuredCfg<N>, StructuringError> {
    let body_nodes = natural_loop_body(header, latches, cfg, dom);

    let mut back_edges = outer_back_edges.clone();
    for latch in latches {
        back_edges.insert((latch.clone(), header.clone()));
    }

    // Collect all exits from the loop body: successors of any body node that are
    // outside the body. This handles loops where the exit edge comes from a latch
    // node rather than the header (e.g. A→B, B→C, C→B, C→D: exit is C→D).
    let mut loop_exit_set: HashSet<N> = HashSet::new();
    for body_node in &body_nodes {
        for s in cfg.successors(body_node).unwrap_or_default() {
            if !body_nodes.contains(s) {
                loop_exit_set.insert(s.clone());
            }
        }
    }
    if loop_exit_set.len() > 1 {
        return Err(StructuringError::Irreducible);
    }
    let loop_exit = loop_exit_set.into_iter().next();

    // Find which forward successor of the header enters the loop body.
    let header_fwd_succs: Vec<N> = cfg
        .successors(header)
        .unwrap_or_default()
        .into_iter()
        .filter(|s| !back_edges.contains(&(header.clone(), (*s).clone())))
        .cloned()
        .collect();

    let in_loop: Vec<N> = header_fwd_succs
        .iter()
        .filter(|s| body_nodes.contains(*s) && *s != header)
        .cloned()
        .collect();

    // For body structuring, stop at the loop exit node so that post-loop nodes
    // are not pulled into the body. Fall back to the header so the back-edge
    // target also acts as a natural stop (it's already filtered by back_edges).
    let body_stop: Option<&N> = loop_exit.as_ref().or(Some(header));

    let body_struct = match in_loop.len() {
        0 => StructuredCfg::Sequence(vec![]),
        1 => structure_from(&in_loop[0], body_stop, cfg, dom, pdom, &back_edges)?,
        _ => return Err(StructuringError::Irreducible),
    };

    let rest = match loop_exit {
        Some(ref exit) => structure_from(exit, stop_at, cfg, dom, pdom, outer_back_edges)?,
        None => StructuredCfg::Sequence(vec![]),
    };

    Ok(flatten(vec![
        StructuredCfg::Loop {
            header: header.clone(),
            body: Box::new(body_struct),
        },
        rest,
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::cfg::PcodeCfg;
    use crate::analysis::cpa::lattice::pcode::PcodeAddressLattice;
    use crate::analysis::cpa::state::PcodeLocation;
    use crate::modeling::machine::cpu::concrete::ConcretePcodeAddress;

    fn n(m: u64) -> ConcretePcodeAddress {
        ConcretePcodeAddress::new(m, 0)
    }

    fn make_cfg(edges: &[(u64, u64)]) -> PcodeCfg<ConcretePcodeAddress, ()> {
        let mut cfg = PcodeCfg::new();
        for &(from, to) in edges {
            cfg.add_edge(n(from), n(to), ());
        }
        cfg
    }

    #[test]
    fn empty_returns_error() {
        let cfg: PcodeCfg<ConcretePcodeAddress, ()> = PcodeCfg::new();
        assert!(matches!(structure(cfg), Err(StructuringError::Empty)));
    }

    #[test]
    fn single_block() {
        let mut cfg: PcodeCfg<ConcretePcodeAddress, ()> = PcodeCfg::new();
        cfg.add_node(n(0));
        let result = structure(cfg).unwrap();
        assert!(matches!(result, StructuredCfg::Block(a) if a == n(0)));
    }

    #[test]
    fn linear_sequence() {
        // A→B→C
        let cfg = make_cfg(&[(0, 1), (1, 2)]);
        let result = structure(cfg).unwrap();
        let StructuredCfg::Sequence(items) = result else {
            panic!("expected Sequence, got {result:?}");
        };
        assert_eq!(items.len(), 3);
        assert!(matches!(&items[0], StructuredCfg::Block(a) if *a == n(0)));
        assert!(matches!(&items[1], StructuredCfg::Block(a) if *a == n(1)));
        assert!(matches!(&items[2], StructuredCfg::Block(a) if *a == n(2)));
    }

    #[test]
    fn diamond_if_else() {
        // A→B, A→C, B→D, C→D
        let cfg = make_cfg(&[(0, 1), (0, 2), (1, 3), (2, 3)]);
        let result = structure(cfg).unwrap();
        let StructuredCfg::Sequence(items) = result else {
            panic!("expected Sequence, got {result:?}");
        };
        assert_eq!(items.len(), 2);
        assert!(matches!(&items[0], StructuredCfg::IfElse { header, .. } if *header == n(0)));
        assert!(matches!(&items[1], StructuredCfg::Block(a) if *a == n(3)));
    }

    #[test]
    fn if_without_else() {
        // A→B, A→C, B→C  (B is the then-branch, C is the merge/else)
        let cfg = make_cfg(&[(0, 1), (0, 2), (1, 2)]);
        let result = structure(cfg).unwrap();
        let StructuredCfg::Sequence(items) = result else {
            panic!("expected Sequence, got {result:?}");
        };
        assert_eq!(items.len(), 2);
        assert!(matches!(&items[0], StructuredCfg::IfElse { header, .. } if *header == n(0)));
        assert!(matches!(&items[1], StructuredCfg::Block(a) if *a == n(2)));
    }

    #[test]
    fn natural_loop() {
        // A→B, B→C, C→B, C→D
        let cfg = make_cfg(&[(0, 1), (1, 2), (2, 1), (2, 3)]);
        let result = structure(cfg).unwrap();
        let StructuredCfg::Sequence(items) = result else {
            panic!("expected Sequence, got {result:?}");
        };
        assert_eq!(items.len(), 3);
        assert!(matches!(&items[0], StructuredCfg::Block(a) if *a == n(0)));
        assert!(matches!(&items[1], StructuredCfg::Loop { header, .. } if *header == n(1)));
        assert!(matches!(&items[2], StructuredCfg::Block(a) if *a == n(3)));

        let StructuredCfg::Loop { body, .. } = &items[1] else {
            panic!()
        };
        assert!(matches!(body.as_ref(), StructuredCfg::Block(a) if *a == n(2)));
    }

    #[test]
    fn while_loop() {
        // A→B, B→C, B→D, C→B
        let cfg = make_cfg(&[(0, 1), (1, 2), (1, 3), (2, 1)]);
        let result = structure(cfg).unwrap();
        let StructuredCfg::Sequence(items) = result else {
            panic!("expected Sequence, got {result:?}");
        };
        assert_eq!(items.len(), 3);
        assert!(matches!(&items[0], StructuredCfg::Block(a) if *a == n(0)));
        assert!(matches!(&items[1], StructuredCfg::Loop { header, .. } if *header == n(1)));
        assert!(matches!(&items[2], StructuredCfg::Block(a) if *a == n(3)));

        let StructuredCfg::Loop { body, .. } = &items[1] else {
            panic!()
        };
        assert!(matches!(body.as_ref(), StructuredCfg::Block(a) if *a == n(2)));
    }

    #[test]
    fn irreducible_fails() {
        // A→B, A→C, B→C, C→B
        let cfg = make_cfg(&[(0, 1), (0, 2), (1, 2), (2, 1)]);
        assert!(matches!(
            structure(cfg),
            Err(StructuringError::Irreducible)
        ));
    }

    #[test]
    fn display_sequence() {
        let cfg = make_cfg(&[(0, 1)]);
        let result = structure(cfg).unwrap();
        assert_eq!(result.to_string(), "block(0:0)\nblock(1:0)\n");
    }

    #[test]
    fn cbranch_then_is_taken_branch() {
        // CBRANCH at n(0): input0 = absolute VarNode pointing to n(2) (taken),
        // fallthrough = n(1). Edges added in CPA order: taken first, then fallthrough.
        // With petgraph head-insertion, successors() returns [n(1), n(2)] (reversed),
        // so without the fix, then=n(1) (fallthrough, WRONG). With the fix, then=n(2).
        use jingle_sleigh::{PcodeOperation, VarNode};
        let cbranch_op = PcodeOperation::CBranch {
            input0: VarNode::new(2u64, 8u32, 1u32), // absolute addr 2 → n(2)
            input1: VarNode::new_const(1u64, 1u32),
        };
        let mut cfg: PcodeCfg<ConcretePcodeAddress, PcodeOperation> = PcodeCfg::new();
        cfg.add_edge(n(0), n(2), cbranch_op.clone()); // taken (added first)
        cfg.add_edge(n(0), n(1), cbranch_op.clone()); // fallthrough (added second)
        cfg.add_edge(n(1), n(3), cbranch_op.clone());
        cfg.add_edge(n(2), n(3), cbranch_op.clone());
        let result = structure(cfg).unwrap();
        let StructuredCfg::Sequence(items) = result else {
            panic!("{result:?}")
        };
        let StructuredCfg::IfElse {
            then_branch,
            else_branch,
            ..
        } = &items[0]
        else {
            panic!("expected IfElse, got {:?}", items[0]);
        };
        assert!(
            matches!(then_branch.as_ref(), StructuredCfg::Block(a) if *a == n(2)),
            "then_branch should be taken (n(2)), got {then_branch:?}"
        );
        assert!(
            matches!(else_branch.as_ref(), StructuredCfg::Block(a) if *a == n(1)),
            "else_branch should be fallthrough (n(1)), got {else_branch:?}"
        );
    }

    #[test]
    fn display_if_else() {
        // A→B, A→C, B→D, C→D
        let cfg = make_cfg(&[(0, 1), (0, 2), (1, 3), (2, 3)]);
        let result = structure(cfg).unwrap();
        let s = result.to_string();
        assert!(s.contains("if 0:0 {"), "missing if header: {s}");
        assert!(s.contains("  block(1:0)"), "missing then-branch: {s}");
        assert!(s.contains("} else {"), "missing else: {s}");
        assert!(s.contains("  block(2:0)"), "missing else-branch: {s}");
        assert!(s.contains("block(3:0)"), "missing merge: {s}");
    }

    #[test]
    fn display_loop() {
        // A→B, B→C, C→B, C→D
        let cfg = make_cfg(&[(0, 1), (1, 2), (2, 1), (2, 3)]);
        let result = structure(cfg).unwrap();
        let s = result.to_string();
        assert!(s.contains("loop 1:0 {"), "missing loop header: {s}");
        assert!(s.contains("  block(2:0)"), "missing loop body: {s}");
        assert!(s.contains("block(3:0)"), "missing post-loop: {s}");
    }

    /// A node type that can represent either a concrete pcode location or a symbolic one.
    /// Used to test that `structure()` prunes symbolic leaf nodes before running.
    #[derive(Clone, Debug, Hash, PartialEq, Eq)]
    enum MaybeSymbolicNode {
        Concrete(ConcretePcodeAddress),
        Symbolic,
    }

    impl PcodeLocation for MaybeSymbolicNode {
        fn location(&self) -> PcodeAddressLattice {
            match self {
                MaybeSymbolicNode::Concrete(addr) => PcodeAddressLattice::Const(*addr),
                MaybeSymbolicNode::Symbolic => PcodeAddressLattice::Top,
            }
        }
    }

    #[test]
    fn symbolic_leaf_is_pruned() {
        // Concrete node A with one successor: a symbolic leaf.
        // Without pruning the structurer sees a Sequence [A, leaf].
        // With pruning it should see just Block(A).
        let concrete = MaybeSymbolicNode::Concrete(ConcretePcodeAddress::new(0, 0));
        let symbolic = MaybeSymbolicNode::Symbolic;
        let mut cfg: PcodeCfg<MaybeSymbolicNode, ()> = PcodeCfg::new();
        cfg.add_edge(concrete.clone(), symbolic, ());
        let result = structure(cfg).unwrap();
        assert!(
            matches!(result, StructuredCfg::Block(ref n) if *n == concrete),
            "expected Block(concrete), got {result:?}"
        );
    }

    #[test]
    fn if_inside_loop() {
        // 0→1 (entry), 1→2 (loop header to if-cond), 1→6 (loop exit),
        // 2→3, 2→4 (if-else branches), 3→5, 4→5 (merge), 5→1 (latch)
        let cfg = make_cfg(&[
            (0, 1),
            (1, 2),
            (1, 6),
            (2, 3),
            (2, 4),
            (3, 5),
            (4, 5),
            (5, 1),
        ]);
        let result = structure(cfg).unwrap();

        let StructuredCfg::Sequence(items) = result else {
            panic!("expected Sequence, got {result:?}");
        };
        assert_eq!(items.len(), 3);
        assert!(matches!(&items[0], StructuredCfg::Block(a) if *a == n(0)));
        assert!(matches!(&items[2], StructuredCfg::Block(a) if *a == n(6)));

        let StructuredCfg::Loop { header, body } = &items[1] else {
            panic!("expected Loop at items[1], got {:?}", items[1]);
        };
        assert_eq!(*header, n(1));

        let StructuredCfg::Sequence(body_items) = body.as_ref() else {
            panic!("expected Sequence in loop body, got {body:?}");
        };
        assert_eq!(body_items.len(), 2);
        assert!(matches!(&body_items[0], StructuredCfg::IfElse { header, .. } if *header == n(2)));
        assert!(matches!(&body_items[1], StructuredCfg::Block(a) if *a == n(5)));
    }
}
