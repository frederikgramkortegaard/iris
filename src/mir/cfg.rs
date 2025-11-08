use crate::mir::{BasicBlock, BlockId, Instruction, MirFunction, MirProgram, Operand, Terminator};

use std::collections::HashMap;

#[derive(Debug)]
pub struct CFGAnalysis {
    pub entry: BlockId,
    pub predecessors: HashMap<BlockId, Vec<BlockId>>,
    pub successors: HashMap<BlockId, Vec<BlockId>>,
}

impl CFGAnalysis {

    pub fn new(function: &MirFunction) -> Self {
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

        CFGAnalysis {
            entry: function.entry,
            predecessors,
            successors,
        }
    }
}
