use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::MirProgram;
use crate::mir::{
    BasicBlock, BlockId, Instruction, MirFunction, MirType, Opcode, Operand, Reg, Terminator,
};
use std::collections::{HashMap, HashSet};

/// Dead Code Elimination pass
pub struct MirTailCallPass {
    diagnostics: DiagnosticCollector,
}

impl Default for MirTailCallPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirTailCallPass {
    pub fn new() -> Self {
        MirTailCallPass {
            diagnostics: DiagnosticCollector::new(),
        }
    }
}

// The visitor is used to mark liveness and build the defmap
impl MirVisitor for MirTailCallPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut MirFunction) {
        for block in &mut function.arena.blocks {
            let Terminator::Ret {
                value: Some(Operand::Reg(r)),
            } = block.terminator
            else {
                continue;
            };

            let Some(inst) = block.instructions.last().cloned() else {
                continue;
            };

            if inst.op != Opcode::Call {
                continue;
            }

            if inst.dest != r {
                continue;
            };

            // Only do Tail Call optim if the function we're calling is ourselves (recursive)
            if let Some(Operand::Label(s)) = inst.args.first() {
                if *s != function.name {
                    continue;
                }
            }

            // Instead of calling, unconditionally go to the function entry
            block.terminator = Terminator::Br {
                target: function.entry,
            };

            block.instructions.pop();

            // Reassign call arguments to function parameters
            for (i, (name, typ)) in function.params.iter().enumerate() {
                block.instructions.push(Instruction {
                    dest: *name,
                    op: Opcode::Copy,
                    args: vec![inst.args[i + 1].clone()],
                    typ: *typ,
                });
            }
        }
    }
}
impl MirPass for MirTailCallPass {
    fn run(&mut self, program: &mut MirProgram) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
