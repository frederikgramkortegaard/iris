use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::{BlockId, Function, Program, Terminator};

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

impl MirVisitor for MirDeadBlockEliminationPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut Function) {
        let dead: Vec<BlockId> = function
            .arena
            .iter()
            .filter(|(id, block)| {
                matches!(block.terminator, Terminator::Unreachable)
                    && *id != function.entry
                    && *id != function.virtual_entry
            })
            .map(|(id, _)| id)
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
