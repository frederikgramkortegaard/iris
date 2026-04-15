use crate::mir::{BlockId, Function, Instruction, Reg};
use std::collections::HashMap;

pub type InstructionIndex = usize;
pub type PhiIndex = usize;

/// Data Flow Graph analysis
/// Maps registers to their defining instructions for O(1) lookup
pub struct DFGAnalysis {
    /// Maps register -> (block, instruction index)
    pub defmap: HashMap<Reg, (BlockId, InstructionIndex)>,
    /// Maps register -> (block, phi index)
    pub phi_defmap: HashMap<Reg, (BlockId, PhiIndex)>,
}

impl DFGAnalysis {
    pub fn compute(function: &Function) -> Self {
        let mut defmap = HashMap::new();
        let mut phi_defmap = HashMap::new();

        for (block_id, block) in function.arena.iter() {
            for (i, phi) in block.phi_nodes.iter().enumerate() {
                phi_defmap.insert(phi.dest, (block_id, i));
            }
            for (i, inst) in block.instructions.iter().enumerate() {
                defmap.insert(inst.dest, (block_id, i));
            }
        }

        Self { defmap, phi_defmap }
    }

    /// Get the instruction that defines a register
    pub fn get_instruction<'a>(&self, function: &'a Function, reg: Reg) -> Option<&'a Instruction> {
        let (block_id, idx) = self.defmap.get(&reg)?;
        Some(&function.arena.get(*block_id).instructions[*idx])
    }

    /// Get the phi node that defines a register
    pub fn get_phi<'a>(&self, function: &'a Function, reg: Reg) -> Option<&'a Instruction> {
        let (block_id, idx) = self.phi_defmap.get(&reg)?;
        Some(&function.arena.get(*block_id).phi_nodes[*idx])
    }

    /// Get the block where a register is defined (either by instruction or phi)
    pub fn get_def_block(&self, reg: Reg) -> Option<BlockId> {
        if let Some((block_id, _)) = self.defmap.get(&reg) {
            return Some(*block_id);
        }
        if let Some((block_id, _)) = self.phi_defmap.get(&reg) {
            return Some(*block_id);
        }
        None
    }
}
