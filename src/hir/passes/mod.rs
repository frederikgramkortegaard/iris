pub mod ast_simplification;
pub mod counting;
pub mod lowering;
pub mod print;
pub mod typechecking;

use crate::ast::Program;
use crate::diagnostics::DiagnosticCollector;

/// Trait for HIR passes that can be run in a pass pipeline.
/// Both visitor-based and non-visitor passes implement this trait.
pub trait HirPass {
    fn run(&mut self, program: &mut Program);
    fn diagnostics(&self) -> &DiagnosticCollector;
}

impl Program {
    /// Run a pass on this program, print diagnostics, and return self for chaining.
    pub fn run_pass<P: HirPass>(
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
