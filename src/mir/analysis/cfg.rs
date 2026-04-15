use crate::mir::{BlockId, Function, Terminator};
use std::collections::HashMap;
use std::collections::HashSet;

pub type Predecessors = HashMap<BlockId, Vec<BlockId>>;
pub type Successors = HashMap<BlockId, Vec<BlockId>>;
pub type DominatorSets = HashMap<BlockId, HashSet<BlockId>>;
pub type DominatorTree = HashMap<BlockId, BlockId>;
pub type DominatorFrontier = HashMap<BlockId, HashSet<BlockId>>;

pub fn compute_cfg(function: &Function) -> (Predecessors, Successors) {
    let mut predecessors: Predecessors = HashMap::new();
    let mut successors: Successors = HashMap::new();

    // Initialize
    for (block_id, _) in function.arena.iter() {
        predecessors.insert(block_id, Vec::new());
        successors.insert(block_id, Vec::new());
    }

    for (block_id, block) in function.arena.iter() {
        match &block.terminator {
            Terminator::Br { target } => {
                // block_id -> target
                successors
                    .get_mut(&block_id)
                    .expect("block pre-inserted")
                    .push(*target);
                predecessors
                    .get_mut(target)
                    .expect("block pre-inserted")
                    .push(block_id);
            }
            Terminator::BrIf {
                then_bb, else_bb, ..
            } => {
                // block_id -> then_bb, else_bb
                successors
                    .get_mut(&block_id)
                    .expect("block pre-inserted")
                    .push(*then_bb);
                successors
                    .get_mut(&block_id)
                    .expect("block pre-inserted")
                    .push(*else_bb);
                predecessors
                    .get_mut(then_bb)
                    .expect("block pre-inserted")
                    .push(block_id);
                predecessors
                    .get_mut(else_bb)
                    .expect("block pre-inserted")
                    .push(block_id);
            }
            _ => {}
        }
    }

    (predecessors, successors)
}

pub fn compute_rpo(entry: BlockId, successors: &Successors) -> Vec<BlockId> {
    let mut visited: HashSet<BlockId> = HashSet::new();
    let mut rpo: Vec<BlockId> = Vec::new();

    // Perform a DFS on the CFG to get the RPO (by reversing the DFS order)
    fn dfs(
        successors: &Successors,
        visited: &mut HashSet<BlockId>,
        rpo: &mut Vec<BlockId>,
        block_id: BlockId,
    ) {
        if visited.contains(&block_id) {
            return;
        }

        visited.insert(block_id);
        for s in successors
            .get(&block_id)
            .expect("block must exist in successors")
        {
            dfs(successors, visited, rpo, *s);
        }
        rpo.push(block_id);
    }

    dfs(successors, &mut visited, &mut rpo, entry);

    rpo
}

/// Iterative data-flow method
/// This generates a dominator set over the function by for each block,
/// calculating the dominators of that block (blocks that we always end up
/// going through if we want to go to this block)
///
/// An example would be
///
/// ```text
///    A
///   / \
///  B   C
///   \ /
///    D
/// ```
///
/// Here, the dominator sets would be as such:
/// - A: {A}
/// - B: {A, B}
/// - C: {A, C}
/// - D: {A, D}
///
/// Looking at node D, you see that only itself (dominator sets are self inclusive)
/// and 'A' is in it's set. This is because you don't _need_ to go through B as you could
/// go through C, and you don't need to go through C as you could go through B.
///
/// We calculate this set by first:
/// 1. Initialize every nodes dominator set to be ALL nodes
/// 2. For every node, set dominators[node] = [Dom[p] intersect for p in predeseccors[node]]
/// 3. Once no change is observed after a calculation, we have stabilized and we are done.
///
/// The intuition is simpler to explain if we consider the types of nodes in the graph:
///
/// - Nodes with no predecessors (e.g. the entry block): their dominator set will only
///   contain themselves. {node}
/// - Nodes with a single predecessor: their dominators will be the strict
///   superset {Dom[pred]} + {node}
/// - Nodes with multiple predecessors: the intersection of all of a nodes predecessors
///   dominator sets, essentially maps to the common ancestors list of node's predecessors.
///
pub fn compute_dominators(function: &Function, predecessors: &Predecessors) -> DominatorSets {
    let mut dom: DominatorSets = HashMap::new();
    let all_blocks: Vec<BlockId> = function.arena.iter().map(|(a, _)| a).collect();

    for &node in &all_blocks {
        if node == function.virtual_entry {
            dom.insert(node, HashSet::from([function.virtual_entry]));
        } else {
            dom.insert(node, HashSet::from_iter(all_blocks.clone()));
        }
    }

    loop {
        let mut changed = false;
        for &node in &all_blocks {
            if node == function.virtual_entry {
                continue;
            }
            let preds = predecessors
                .get(&node)
                .expect("block must have predecessors entry");

            if preds.is_empty() {
                continue;
            }

            let mut inter: HashSet<BlockId> = dom
                .get(&preds[0])
                .expect("predecessor must have dom set")
                .clone();
            for &p in &preds[1..] {
                inter.retain(|x| {
                    dom.get(&p)
                        .expect("predecessor must have dom set")
                        .contains(x)
                });
            }

            inter.insert(node);

            if inter != dom[&node] {
                changed = true;
                dom.insert(node, inter);
            }
        }

        if !changed {
            break;
        }
    }
    dom
}

// @TODO write intuition
pub fn compute_dominator_frontier(
    dtree: &DominatorTree,
    predecessors: &Predecessors,
) -> DominatorFrontier {
    let mut dfront: DominatorFrontier = DominatorFrontier::new();

    for (block, preds) in predecessors {
        if preds.len() < 2 {
            continue;
        }

        for &pred in preds {
            let mut runner = pred;
            while runner != dtree[block] {
                dfront.entry(runner).or_default().insert(*block);
                runner = dtree[&runner];
            }
        }
    }

    dfront
}

pub fn compute_dominator_tree(
    function: &Function,
    dominators: &DominatorSets,
    successors: &Successors,
) -> DominatorTree {
    let mut idom = DominatorTree::new();
    let reachable: HashSet<BlockId> = compute_rpo(function.virtual_entry, successors)
        .into_iter()
        .collect();

    for (&block, doms) in dominators {
        if block == function.virtual_entry || !reachable.contains(&block) {
            continue;
        }

        // Calculate Strict Dominators
        let strict_doms: Vec<_> = doms.iter().filter(|&&d| d != block).collect();

        // For each candidate, check if it dominates another strict dominator, if it does, then it
        // is not the imediate dominator.
        // INTUITION:
        //
        for &candidate in &strict_doms {
            let dominated_another: bool = strict_doms
                .iter()
                .any(|&other| other != candidate && dominators[other].contains(candidate));

            if !dominated_another {
                idom.insert(block, *candidate);
            }
        }
    }

    idom
}

pub fn dominated_subtree(dtree: &DominatorTree, root: BlockId) -> HashSet<BlockId> {
    let mut dominated = HashSet::new();
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        dominated.insert(node);
        for (&child, &parent) in dtree {
            if parent == node && !dominated.contains(&child) {
                stack.push(child);
            }
        }
    }

    dominated
}
