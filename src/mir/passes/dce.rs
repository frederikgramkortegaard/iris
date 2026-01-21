use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::MirProgram;

/// Dead Code Elimination pass
pub struct MirDCEPass {
    diagnostics: DiagnosticCollector,
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
        }
    }
}

impl MirPass for MirDCEPass {
    fn run(&mut self, _program: &mut MirProgram) {
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
