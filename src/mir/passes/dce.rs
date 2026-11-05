use crate::diagnostics::DiagnosticCollector;
use crate::mir::analysis::dfg::DFGAnalysis;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::Program;
use crate::mir::{BasicBlock, BlockId, Function, Opcode, Operand, Reg, Terminator, Type};
use log::debug;
use std::collections::HashSet;

/// Dead Code Elimination pass
pub struct MirDCEPass {
    diagnostics: DiagnosticCollector,
    live: HashSet<Reg>,
    worklist: Vec<Reg>,
    changed: bool,
}

impl Default for MirDCEPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirDCEPass {
    pub fn new() -> Self {
        MirDCEPass {
            diagnostics: DiagnosticCollector::new(),
            live: HashSet::new(),
            worklist: vec![],
            changed: false,
        }
    }

    /// Whether the last run modified the program
    pub fn changed(&self) -> bool {
        self.changed
    }

    fn has_side_effects(&self, op: &Opcode) -> bool {
        matches!(op, Opcode::Call)
    }

    fn propagate_worklist(&mut self, function: &Function, dfg: &DFGAnalysis) {
        let live = &mut self.live;
        let worklist = &mut self.worklist;
        while let Some(reg) = worklist.pop() {
            let defs = [
                dfg.get_instruction(function, reg),
                dfg.get_phi(function, reg),
            ];
            for inst in defs.into_iter().flatten() {
                for arg in &inst.args {
                    arg.for_each_reg(&mut |r| {
                        if live.insert(r) {
                            worklist.push(r);
                        }
                    });
                }
            }
        }
    }
    fn sweep(&mut self, function: &mut Function) {
        let mut removed = false;
        for (_, block) in function.arena.iter_mut() {
            block.instructions.retain(|inst| {
                let keep = self.live.contains(&inst.dest);
                if !keep {
                    debug!("Removing Instruction {:?} from block", inst);
                    removed = true;
                }
                keep
            });
            block.phi_nodes.retain(|phi| {
                let keep = self.live.contains(&phi.dest);
                if !keep {
                    debug!("Removing dead Phi {:?} from block", phi);
                    removed = true;
                }
                keep
            });
        }
        self.changed = self.changed || removed;
    }
}

// The visitor is used to mark the liveness roots
impl MirVisitor for MirDCEPass {
    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut Function) {
        self.live.clear();
        self.worklist.clear();

        self.walk_function(function);
        debug!(
            "Function: {}:\nlive: {:?}\nworklist: {:?}\n\n",
            function.name, self.live, self.worklist
        );

        let dfg = DFGAnalysis::compute(function);
        self.propagate_worklist(function, &dfg);

        self.sweep(function);
        debug!(
            "Function: {}:\nlive: {:?}\nworklist: {:?}\n\n",
            function.name, self.live, self.worklist
        );
    }

    fn visit_basicblock(&mut self, _block_id: BlockId, block: &mut BasicBlock) {
        for instruction in block.instructions.iter() {
            // If the instruction has (or could have) side effects, we assume the dest is live,
            // because the defining line itself can't be known to be safe to delete at this point
            if self.has_side_effects(&instruction.op) {
                self.live.insert(instruction.dest);

                // Because of this, all of the arguments must also be live
                for arg in &instruction.args {
                    if let Operand::Reg(r) = arg {
                        // If the value does NOT exist in live, we add it to the worklist. If it
                        // didnt exist there, it means its the first time we're marking R as
                        // live and thus we should process it
                        if self.live.insert(*r) {
                            self.worklist.push(*r);
                        }
                    }
                }
            }
        }

        self.walk_basicblock(block);
    }

    fn visit_terminator(&mut self, term: &mut Terminator) {
        match &term {
            Terminator::Ret {
                value: Some(Operand::Reg(r)),
            } if self.live.insert(*r) => {
                self.worklist.push(*r);
            }
            Terminator::BrIf {
                cond: Operand::Reg(r),
                ..
            } if self.live.insert(*r) => {
                self.worklist.push(*r);
            }
            _ => {}
        }
    }

    fn visit_param(&mut self, reg: Reg, _type: Type) {
        // Parameters are special, they are ALWAYS considered live @TODO : Maybe it would be
        // possible to do some weird optimization here
        self.live.insert(reg);
    }
}
impl MirPass for MirDCEPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
