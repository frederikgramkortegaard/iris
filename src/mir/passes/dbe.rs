use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::{BlockId, Function, Instruction, Opcode, Operand, Program, Terminator};
use std::collections::{BTreeSet, HashSet};

pub struct MirDeadBlockEliminationPass {
    diagnostics: DiagnosticCollector,
    changed: bool,
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
            changed: false,
        }
    }

    /// Whether the last run modified the program
    pub fn changed(&self) -> bool {
        self.changed
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
                    self.changed = true;
                }
            }
        }

        // Then eliminate unreachable blocks
        let reachable = reachable_blocks(function);

        let dead: BTreeSet<BlockId> = function
            .arena
            .iter()
            .map(|(id, _)| id)
            .filter(|id| !reachable.contains(id))
            .collect();

        if dead.is_empty() {
            return;
        }
        self.changed = true;

        for &id in &dead {
            function.arena.remove(id);
        }

        // Phis in surviving blocks can still name a removed predecessor,
        // and phi elimination would then try to insert copies into a block
        // that no longer exists. Drop those entries. A phi left with one
        // entry is not a join anymore, it's just a copy.
        for (_, block) in function.arena.iter_mut() {
            let mut demoted: Vec<Instruction> = Vec::new();
            block.phi_nodes.retain_mut(|phi| {
                phi.args
                    .retain(|arg| !matches!(arg, Operand::Pair(b, _) if dead.contains(b)));
                match phi.args.as_slice() {
                    [] => false,
                    [Operand::Pair(_, value)] => {
                        demoted.push(Instruction {
                            dest: phi.dest,
                            op: Opcode::Copy,
                            typ: phi.typ,
                            args: vec![(**value).clone()],
                        });
                        false
                    }
                    _ => true,
                }
            });
            for (i, inst) in demoted.into_iter().enumerate() {
                block.instructions.insert(i, inst);
            }
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
