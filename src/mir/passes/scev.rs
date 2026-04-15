use crate::diagnostics::DiagnosticCollector;
use crate::mir::analysis::dfg::DFGAnalysis;
use crate::mir::analysis::iv;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::{Function, Program};

pub struct MirSCEVPass {
    diagnostics: DiagnosticCollector,
}

impl Default for MirSCEVPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirSCEVPass {
    pub fn new() -> Self {
        MirSCEVPass {
            diagnostics: DiagnosticCollector::new(),
        }
    }
}

impl MirVisitor for MirSCEVPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        let Some(loops) = function.loops.as_ref() else {
            return;
        };

        if loops.is_empty() {
            return;
        }

        let dfg = DFGAnalysis::compute(function);

        for lop in loops {
            let scevs = iv::compute(function, lop, &dfg);
            for (reg, addrec) in &scevs {
                println!("r{}: {:?}", reg, addrec);
            }
        }
    }
}

impl MirPass for MirSCEVPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
