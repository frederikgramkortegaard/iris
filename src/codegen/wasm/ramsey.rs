use crate::codegen::wasm::StructuredNode;
use crate::mir::analysis::cfg::{DominatorTree, Successors};
use crate::mir::BlockId;
use std::collections::HashSet;

/// Checks if `block_id` has a back edge to a node inside `region` using dominator info.
pub fn has_back_edge_in_region(
    block_id: BlockId,
    succs: &Successors,
    region: &HashSet<BlockId>,
    doms: &DominatorTree,
) -> bool {
    for &succ in &succs[&block_id] {
        if region.contains(&succ) && doms[&block_id] == succ {
            return true;
        }
    }
    false
}

/// Recursively structures a CFG into canonical structured nodes (loops, if-else, sequences).
pub fn ramsey_structuring(
    block_id: BlockId,
    dom_tree: &DominatorTree,
    succs: &Successors,
) -> StructuredNode {
    // Region dominated by this block (subtree)
    let region: HashSet<BlockId> = crate::mir::analysis::cfg::dominated_subtree(dom_tree, block_id);

    // Successors of this block that are inside the region
    let outs: Vec<_> = succs[&block_id]
        .iter()
        .filter(|&&succ| region.contains(&succ))
        .collect();

    // This block is a loop header if any successor has a back edge to it
    let has_loop_edge = outs
        .iter()
        .any(|&&succ| has_back_edge_in_region(succ, succs, &region, dom_tree));

    if has_loop_edge {
        // This block is a loop header

        // Separate body successors (reach back to header) from exit successors
        let mut body_succs = Vec::new();
        let mut exit_succs = Vec::new();
        for &&s in &outs {
            if has_back_edge_in_region(s, succs, &region, dom_tree) {
                body_succs.push(s);
            } else {
                exit_succs.push(s);
            }
        }

        // Structure the loop body
        let body = if body_succs.len() == 1 {
            ramsey_structuring(body_succs[0], dom_tree, succs)
        } else {
            let nodes: Vec<_> = body_succs
                .iter()
                .map(|&s| ramsey_structuring(s, dom_tree, succs))
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
            let exit = ramsey_structuring(exit_succs[0], dom_tree, succs);
            StructuredNode::Sequence(vec![loop_node, exit])
        } else {
            let mut seq = vec![loop_node];
            for s in exit_succs {
                seq.push(ramsey_structuring(s, dom_tree, succs));
            }
            StructuredNode::Sequence(seq)
        }
    } else if outs.len() == 2 {
        // This block is an if-else

        let then_branch = ramsey_structuring(*outs[0], dom_tree, succs);
        let else_branch = ramsey_structuring(*outs[1], dom_tree, succs);
        StructuredNode::If {
            cond: block_id,
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        }
    } else if outs.len() == 1 {
        // This block is straight-line code

        let next = ramsey_structuring(*outs[0], dom_tree, succs);
        StructuredNode::Sequence(vec![StructuredNode::Block(block_id), next])
    } else {
        // No successors in region; leaf node

        StructuredNode::Block(block_id)
    }
}
