use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::{Function, Instruction, Opcode, Operand, Program, Reg, Terminator};
use log::debug;
use std::collections::HashMap;

pub struct MirCopyPropPass {
    diagnostics: DiagnosticCollector,
    copy_map: HashMap<Reg, Reg>,
}

impl Default for MirCopyPropPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirCopyPropPass {
    pub fn new() -> Self {
        MirCopyPropPass {
            diagnostics: DiagnosticCollector::new(),
            copy_map: HashMap::new(),
        }
    }
}

impl MirVisitor for MirCopyPropPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        self.walk_function(function);

        // Before we can do proper copy-prop for phi nodes, we need to fully fill out the copy map.
        // Thus we have to do it after we've already visited all blocks and functions.
        for (_, block) in function.arena.iter_mut() {
            for phi in &mut block
                .phi_nodes
                .iter_mut()
                .filter(|inst| inst.op == Opcode::Phi)
            {
                for arg in &mut phi.args {
                    if let Operand::Pair(_, op) = arg {
                        if let Operand::Reg(src) = op.as_mut() {
                            if let Some(&r) = self.copy_map.get(src) {
                                debug!(
                                    "Replacing (phi) register r{} with copy r{}",
                                    src, r
                                );
                                *src = r; // mutate the register inside, keep the Pair
                            }
                        }
                    }
                }
            }
        }
        self.copy_map.clear();
    }

    fn visit_instruction(&mut self, instruction: &mut Instruction) -> Self::Output {
        for arg in &mut instruction.args {
            if let Operand::Reg(src) = arg {
                if let Some(&r) = self.copy_map.get(src) {
                    debug!("Replacing register r{} with copy r{}", src, r);
                    *arg = Operand::Reg(r)
                }
            }
        }

        if instruction.op == Opcode::Copy {
            if let Some(Operand::Reg(src)) = instruction.args.first() {
                self.copy_map.insert(instruction.dest, *src);
            }
        }
    }

    fn visit_terminator(&mut self, term: &mut Terminator) -> Self::Output {
        match term {
            Terminator::Ret {
                value: Some(Operand::Reg(r)),
            } => {
                if let Some(&src) = self.copy_map.get(r) {
                    *r = src
                }
            }
            Terminator::BrIf {
                cond: Operand::Reg(r),
                ..
            } => {
                if let Some(&src) = self.copy_map.get(r) {
                    *r = src
                }
            }
            _ => {}
        }
    }
}

impl MirPass for MirCopyPropPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
