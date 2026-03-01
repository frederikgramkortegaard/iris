use crate::diagnostics::DiagnosticCollector;
use crate::mir::visitor::MirVisitor;
use crate::mir::{Function, Program};
use crate::pass::PassWithOutput;

/// A placeholder WASM instruction type — replace with a real encoding type later.
#[derive(Debug)]
pub enum WasmInstruction {
    Unreachable,
}

pub struct MirToWasmPass {
    diagnostics: DiagnosticCollector,
    output: Vec<WasmInstruction>,
}

impl Default for MirToWasmPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirToWasmPass {
    pub fn new() -> Self {
        MirToWasmPass {
            diagnostics: DiagnosticCollector::new(),
            output: Vec::new(),
        }
    }
}

impl MirVisitor for MirToWasmPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        self.walk_function(function);
    }
}

impl PassWithOutput<Program> for MirToWasmPass {
    type Output = Vec<WasmInstruction>;

    fn run(&mut self, program: &mut Program) -> Self::Output {
        self.visit_program(program);
        std::mem::take(&mut self.output)
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
