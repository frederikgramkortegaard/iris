use crate::codegen::wasm::StructuredNode;
use crate::mir::analysis::cfg::{DominatorTree, Predecessors, Successors};
use crate::mir::BlockId;
use std::collections::HashSet;

/// True if any block dominated by start branches back to header.
/// Everything start dominates is also dominated by header, so such an edge
/// is always a back edge. The latch can sit arbitrarily deep (below an if
/// in the loop body for example), checking only direct successors misses
/// those loops entirely.
pub fn reaches_back_to(
    header: BlockId,
    start: BlockId,
    succs: &Successors,
    doms: &DominatorTree,
) -> bool {
    let subtree = crate::mir::analysis::cfg::dominated_subtree(doms, start);
    subtree
        .iter()
        .any(|b| succs.get(b).is_some_and(|ss| ss.contains(&header)))
}

/// Recursively structures a CFG into canonical structured nodes (loops, if-else, sequences).
pub fn ramsey_structuring(
    block_id: BlockId,
    dom_tree: &DominatorTree,
    succs: &Successors,
    preds: &Predecessors,
) -> StructuredNode {
    // Region dominated by this block (subtree)
    let region: HashSet<BlockId> = crate::mir::analysis::cfg::dominated_subtree(dom_tree, block_id);

    // Successors of this block that are inside the region
    let outs: Vec<_> = succs[&block_id]
        .iter()
        .filter(|&&succ| region.contains(&succ))
        .collect();

    // This block is a loop header if any successor can reach back to it
    let has_loop_edge = outs
        .iter()
        .any(|&&succ| reaches_back_to(block_id, succ, succs, dom_tree));

    if has_loop_edge {
        // This block is a loop header

        // Separate body successors (reach back to header) from exit successors
        let mut body_succs = Vec::new();
        let mut exit_succs = Vec::new();
        for &&s in &outs {
            if reaches_back_to(block_id, s, succs, dom_tree) {
                body_succs.push(s);
            } else {
                exit_succs.push(s);
            }
        }

        // Structure the loop body
        let body = if body_succs.len() == 1 {
            ramsey_structuring(body_succs[0], dom_tree, succs, preds)
        } else {
            let nodes: Vec<_> = body_succs
                .iter()
                .map(|&s| ramsey_structuring(s, dom_tree, succs, preds))
                .collect();
            StructuredNode::Sequence(nodes)
        };

        let loop_node = StructuredNode::Loop {
            header: block_id,
            body: Box::new(body),
        };

        // Exit successors come after the loop
        if exit_succs.is_empty() {
            loop_node
        } else if exit_succs.len() == 1 {
            let exit = ramsey_structuring(exit_succs[0], dom_tree, succs, preds);
            StructuredNode::Sequence(vec![loop_node, exit])
        } else {
            let mut seq = vec![loop_node];
            for s in exit_succs {
                seq.push(ramsey_structuring(s, dom_tree, succs, preds));
            }
            StructuredNode::Sequence(seq)
        }
    } else if outs.len() == 2 {
        // This block is an if-else. Careful: a successor with >1 preds is
        // not an arm, it's the join block (empty arm case, `if (c) { x = 0 }`).
        // That arm becomes empty and the join goes after the if.
        let then_bb = *outs[0];
        let else_bb = *outs[1];
        let is_join = |b: BlockId| preds.get(&b).is_some_and(|p| p.len() > 1);

        let empty = || Box::new(StructuredNode::Sequence(vec![]));
        let (then_branch, else_branch, arms) = match (is_join(then_bb), is_join(else_bb)) {
            (true, false) => (
                empty(),
                Box::new(ramsey_structuring(else_bb, dom_tree, succs, preds)),
                vec![else_bb],
            ),
            (false, true) => (
                Box::new(ramsey_structuring(then_bb, dom_tree, succs, preds)),
                empty(),
                vec![then_bb],
            ),
            _ => (
                Box::new(ramsey_structuring(then_bb, dom_tree, succs, preds)),
                Box::new(ramsey_structuring(else_bb, dom_tree, succs, preds)),
                vec![then_bb, else_bb],
            ),
        };

        let if_node = StructuredNode::If {
            cond: block_id,
            then_branch,
            else_branch,
        };

        // The join is a dom-tree child of the condition, not a CFG successor,
        // so it has to be emitted here, after the if. Both arms fall through
        // into it.
        let continuations: Vec<BlockId> = dom_tree
            .iter()
            .filter(|(_, &parent)| parent == block_id)
            .map(|(&child, _)| child)
            .filter(|c| !arms.contains(c) && is_join(*c))
            .collect();

        if continuations.is_empty() {
            if_node
        } else {
            let mut seq = vec![if_node];
            for c in continuations {
                seq.push(ramsey_structuring(c, dom_tree, succs, preds));
            }
            StructuredNode::Sequence(seq)
        }
    } else if outs.len() == 1 {
        // This block is straight-line code

        let next = ramsey_structuring(*outs[0], dom_tree, succs, preds);
        StructuredNode::Sequence(vec![StructuredNode::Block(block_id), next])
    } else {
        // No successors in region; leaf node

        StructuredNode::Block(block_id)
    }
}
