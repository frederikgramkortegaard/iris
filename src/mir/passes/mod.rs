pub mod const_prop;
pub mod dce;
pub mod print;
pub mod ssa;

use crate::diagnostics::DiagnosticCollector;
use crate::mir::MirProgram;

/// Trait for MIR passes that can be run in a pass pipeline.
/// Both visitor-based and worklist-based passes implement this trait.
pub trait MirPass {
    fn run(&mut self, program: &mut MirProgram);
    fn diagnostics(&self) -> &DiagnosticCollector;
}

impl MirProgram {
    /// Run a pass on this program, print diagnostics, and return self for chaining.
    pub fn run_pass<P: MirPass>(
        &mut self,
        pass: &mut P,
    ) -> Result<&mut Self, Box<dyn std::error::Error>> {
        pass.run(self);

        let diagnostics = pass.diagnostics();

        for error in &diagnostics.errors {
            eprintln!("Error: {}", error);
        }
        for warning in &diagnostics.warnings {
            eprintln!("Warning: {}", warning);
        }
        for info in &diagnostics.info {
            println!("Info: {}", info);
        }

        if diagnostics.has_errors() {
            return Err("Compilation failed due to errors".into());
        }

        Ok(self)
    }
}
