use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::{Function, Program, Reg};
use std::collections::{BTreeSet, HashMap};

pub struct RegCompactPass {
    diagnostics: DiagnosticCollector,
}

impl Default for RegCompactPass {
    fn default() -> Self {
        Self::new()
    }
}

impl RegCompactPass {
    pub fn new() -> Self {
        RegCompactPass {
            diagnostics: DiagnosticCollector::new(),
        }
    }

    fn compact_function(&self, function: &mut Function) {
        let mut used = BTreeSet::new();

        for &(reg, _) in &function.params {
            used.insert(reg);
        }

        for (_, block) in function.arena.iter() {
            for inst in &block.instructions {
                used.insert(inst.dest);
                for arg in &inst.args {
                    arg.for_each_reg(&mut |r| {
                        used.insert(r);
                    });
                }
            }
            for op in block.terminator.operands() {
                op.for_each_reg(&mut |r| {
                    used.insert(r);
                });
            }
        }

        // Build old -> new mapping (BTreeSet gives sorted order)
        let mapping: HashMap<Reg, Reg> = used
            .iter()
            .enumerate()
            .map(|(new, &old)| (old, new))
            .collect();

        // Rewrite params
        for param in &mut function.params {
            param.0 = mapping[&param.0];
        }

        // Rewrite all blocks
        for (_, block) in function.arena.iter_mut() {
            for inst in &mut block.instructions {
                inst.dest = mapping[&inst.dest];
                for arg in &mut inst.args {
                    arg.map_regs(&mut |r| mapping[&r]);
                }
            }
            for op in block.terminator.operands_mut() {
                op.map_regs(&mut |r| mapping[&r]);
            }
        }

        function.next_free_reg = used.len();
    }
}

impl MirPass for RegCompactPass {
    fn run(&mut self, program: &mut Program) {
        for (_, function) in program.functions.iter_mut() {
            self.compact_function(function);
        }
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
