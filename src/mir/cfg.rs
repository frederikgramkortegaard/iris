use crate::mir::{BasicBlock, BlockId, Instruction, MirFunction, MirProgram, Operand, Terminator};

use std::collections::HashMap;

use std::collections::HashSet;

#[derive(Debug)]
pub struct CFGAnalysis {
    pub entry: BlockId,
    pub predecessors: HashMap<BlockId, Vec<BlockId>>,
    pub successors: HashMap<BlockId, Vec<BlockId>>,
    pub rpo: Vec<BlockId>,
}

impl CFGAnalysis {
    fn compute_preds_succs(
        function: &MirFunction,
    ) -> (
        HashMap<BlockId, Vec<BlockId>>,
        HashMap<BlockId, Vec<BlockId>>,
    ) {
        let mut predecessors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        let mut successors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();

        // Initialize
        for (block_id, _) in function.arena.iter() {
            predecessors.insert(block_id, Vec::new());
            successors.insert(block_id, Vec::new());
        }

        for (block_id, block) in function.arena.iter() {
            match &block.terminator {
                Terminator::Br { target } => {
                    // block_id -> target
                    successors.get_mut(&block_id).unwrap().push(*target);
                    predecessors.get_mut(target).unwrap().push(block_id);
                }
                Terminator::BrIf {
                    then_bb, else_bb, ..
                } => {
                    // block_id -> then_bb, else_bb
                    successors.get_mut(&block_id).unwrap().push(*then_bb);
                    successors.get_mut(&block_id).unwrap().push(*else_bb);
                    predecessors.get_mut(then_bb).unwrap().push(block_id);
                    predecessors.get_mut(else_bb).unwrap().push(block_id);
                }
                _ => {}
            }
        }

        (predecessors, successors)
    }

    fn compute_rpo(entry: &BlockId, successors: &HashMap<BlockId, Vec<BlockId>>) -> Vec<BlockId> {
        let mut visited: HashSet<BlockId> = HashSet::new();
        let mut rpo: Vec<BlockId> = Vec::new();

        // Perform a DFS on the CFG to get the RPO (by reversing the DFS order)
        fn dfs(
            successors: &HashMap<BlockId, Vec<BlockId>>,
            visited: &mut HashSet<BlockId>,
            rpo: &mut Vec<BlockId>,
            block_id: &BlockId,
        ) {
            if visited.get(block_id).is_some() {
                return;
            }

            visited.insert(*block_id);
            for s in successors.get(block_id).unwrap() {
                dfs(successors, visited, rpo, s);
            }
            rpo.push(*block_id);
        }

        dfs(successors, &mut visited, &mut rpo, entry);

        rpo
    }

    //
    fn compute_dominator_tree(
        entry: &BlockId,
        predecessors: &HashMap<BlockId, Vec<BlockId>>,
        rpo: &Vec<BlockId>,
    ) -> HashMap<BlockId, Vec<BlockId>> {
        let mut idoms: HashMap<BlockId, Option<BlockId>> = HashMap::new();
        idoms.insert(*entry, Some(*entry));

        let mut changed = true;
        while changed {
            changed = false;

            for b in &rpo[1..] {
                let new_idom = predecessors.get(&b).and_then(|ps| {
                    ps.iter()
                        .find(|p| idoms.get(p).is_some_and(|x| x.is_some()))
                        .copied()
                });

                for pred in predecessors.get(b).unwrap() {}
            }
        }

        HashMap::new()
    }

    pub fn new(function: &MirFunction) -> Self {
        let (predecessors, successors) = CFGAnalysis::compute_preds_succs(function);
        let rpo = CFGAnalysis::compute_rpo(&function.entry, &successors);

        CFGAnalysis {
            entry: function.entry,
            predecessors,
            successors,
            rpo,
        }
    }
}
