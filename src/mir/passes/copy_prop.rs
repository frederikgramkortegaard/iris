use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::{Instruction, MirFunction, MirProgram, Opcode, Operand, Reg};
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

    fn visit_function(&mut self, function: &mut MirFunction) -> Self::Output {
        self.walk_function(function);
        self.copy_map.clear();
    }

    fn visit_instruction(&mut self, instruction: &mut Instruction) -> Self::Output {
        if !matches!(instruction.op, Opcode::Copy) {
            self.walk_instruction(instruction);
            return;
        }

        if instruction.args.len() != 1 {
            println!("{:?}", instruction);
            self.diagnostics.error("An instruction with Opcode::Copy should not be able to have any other number of arguments than 1".into());
        }

        if let Some(Operand::Reg(src)) = instruction.args.first() {
            if let Some(&r) = self.copy_map.get(src) {
                instruction.args[0] = Operand::Reg(r);
            }
        }

        if let Some(Operand::Reg(src)) = instruction.args.first() {
            self.copy_map.insert(instruction.dest, *src);
        }
    }
}

impl MirPass for MirCopyPropPass {
    fn run(&mut self, program: &mut MirProgram) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
