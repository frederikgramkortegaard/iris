use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::{BlockId, Function, Operand, Program, Terminator};
use std::collections::HashSet;

pub struct MirDeadBlockEliminationPass {
    diagnostics: DiagnosticCollector,
}

impl Default for MirDeadBlockEliminationPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirDeadBlockEliminationPass {
    pub fn new() -> Self {
        MirDeadBlockEliminationPass {
            diagnostics: DiagnosticCollector::new(),
        }
    }
}

/// Walk reachable blocks from a starting block.
fn reachable_blocks(function: &Function) -> HashSet<BlockId> {
    let mut visited = HashSet::new();
    let mut worklist = vec![function.virtual_entry];

    while let Some(block_id) = worklist.pop() {
        if !visited.insert(block_id) {
            continue;
        }
        let block = function.arena.get(block_id);
        match &block.terminator {
            Terminator::Br { target } => {
                worklist.push(*target);
            }
            Terminator::BrIf {
                cond,
                then_bb,
                else_bb,
            } => match cond {
                Operand::ImmBool(true) => worklist.push(*then_bb),
                Operand::ImmBool(false) => worklist.push(*else_bb),
                _ => {
                    worklist.push(*then_bb);
                    worklist.push(*else_bb);
                }
            },
            Terminator::Ret { .. } | Terminator::Unreachable => {}
        }
    }

    visited
}

impl MirVisitor for MirDeadBlockEliminationPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut Function) {
        // First, simplify constant BrIf -> Br
        for (_, block) in function.arena.iter_mut() {
            if let Terminator::BrIf {
                cond,
                then_bb,
                else_bb,
            } = &block.terminator
            {
                let new_term = match cond {
                    Operand::ImmBool(true) => Some(Terminator::Br { target: *then_bb }),
                    Operand::ImmBool(false) => Some(Terminator::Br { target: *else_bb }),
                    _ => None,
                };
                if let Some(t) = new_term {
                    block.terminator = t;
                }
            }
        }

        // Then eliminate unreachable blocks
        let reachable = reachable_blocks(function);

        let dead: Vec<BlockId> = function
            .arena
            .iter()
            .map(|(id, _)| id)
            .filter(|id| !reachable.contains(id))
            .collect();

        for id in dead {
            function.arena.remove(id);
        }
    }
}

impl MirPass for MirDeadBlockEliminationPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
