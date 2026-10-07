use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::deconstruct::sequentialize_parallel_copies;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::Program;
use crate::mir::{BlockId, Function, Opcode, Operand, Reg, Terminator, Type};

/// Tail call optimization: rewrites a self-recursive call in return position
/// into copies to the parameters and a jump back to the entry block.
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

impl MirVisitor for MirTailCallPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut Function) {
        let block_ids: Vec<BlockId> = function.arena.iter().map(|(id, _)| id).collect();

        for block_id in block_ids {
            let block = function.arena.get(block_id);

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

            // Reassign call arguments to the parameters. These are parallel
            // copies, f(b, a) would clobber a param another copy still
            // reads, so run them through the same sequentialization as phi
            // elimination.
            let copies: Vec<(Reg, Operand, Type)> = function
                .params
                .iter()
                .enumerate()
                .map(|(i, (param, typ))| (*param, inst.args[i + 1].clone(), *typ))
                .collect();
            let sequenced = sequentialize_parallel_copies(&copies, &mut function.next_free_reg);

            // Instead of calling, unconditionally go to the function entry
            let entry = function.entry;
            let block = function.arena.get_mut(block_id);
            block.terminator = Terminator::Br { target: entry };
            block.instructions.pop();
            block.instructions.extend(sequenced);
        }
    }
}
impl MirPass for MirTailCallPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
