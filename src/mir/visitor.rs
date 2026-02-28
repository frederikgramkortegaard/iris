use crate::mir::{
    BasicBlock, BlockId, Function, Instruction, Operand, Program, Reg, Terminator, Type,
};

// Re-export DiagnosticCollector for convenience
pub use crate::diagnostics::DiagnosticCollector;

/// Visitor trait for traversing the MIR
pub trait MirVisitor {
    /// The type returned by visitor methods
    type Output: Default;

    /// Returns the diagnostic collector for this visitor
    fn diagnostics(&self) -> &DiagnosticCollector;

    /// Returns a mutable reference to the diagnostic collector
    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector;

    // Program
    fn visit_program(&mut self, program: &mut Program) -> Self::Output {
        self.walk_program(program)
    }

    fn walk_program(&mut self, program: &mut Program) -> Self::Output {
        for function in &mut program.functions {
            self.visit_function(function);
        }
        Self::Output::default()
    }

    // Function
    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        self.walk_function(function)
    }

    fn walk_function(&mut self, function: &mut Function) -> Self::Output {
        // Visit parameters
        for (reg, typ) in function.params.clone() {
            self.visit_param(reg, typ);
        }

        // Collect block IDs sorted for deterministic output
        let mut block_ids: Vec<BlockId> = function.arena.iter().map(|(id, _)| id).collect();
        block_ids.sort_by_key(|id| id.index());

        // Visit each block
        for block_id in block_ids {
            let block = function.arena.get_mut(block_id);
            self.visit_basicblock(block_id, block);
        }
        Self::Output::default()
    }

    // Param
    fn visit_param(&mut self, reg: Reg, typ: Type) -> Self::Output {
        self.walk_param(reg, typ)
    }

    fn walk_param(&mut self, _reg: Reg, _typ: Type) -> Self::Output {
        Self::Output::default()
    }

    // BasicBlock
    fn visit_basicblock(&mut self, _block_id: BlockId, block: &mut BasicBlock) -> Self::Output {
        self.walk_basicblock(block)
    }

    fn walk_basicblock(&mut self, block: &mut BasicBlock) -> Self::Output {
        // Phi nodes come first in SSA form
        for phi in &mut block.phi_nodes {
            self.visit_instruction(phi);
        }
        for instruction in &mut block.instructions {
            self.visit_instruction(instruction);
        }
        self.visit_terminator(&mut block.terminator);
        Self::Output::default()
    }

    // Instruction
    fn visit_instruction(&mut self, instruction: &mut Instruction) -> Self::Output {
        self.walk_instruction(instruction)
    }

    fn walk_instruction(&mut self, instruction: &mut Instruction) -> Self::Output {
        for arg in &mut instruction.args {
            self.visit_operand(arg);
        }
        Self::Output::default()
    }

    // Terminator
    fn visit_terminator(&mut self, terminator: &mut Terminator) -> Self::Output {
        self.walk_terminator(terminator)
    }

    fn walk_terminator(&mut self, terminator: &mut Terminator) -> Self::Output {
        match terminator {
            Terminator::BrIf { cond, .. } => {
                self.visit_operand(cond);
            }
            Terminator::Ret {
                value: Some(operand),
            } => {
                self.visit_operand(operand);
            }
            _ => {}
        }
        Self::Output::default()
    }

    // Operand (leaf node, useful for analysis)
    fn visit_operand(&mut self, _operand: &mut Operand) -> Self::Output {
        Self::Output::default()
    }
}
