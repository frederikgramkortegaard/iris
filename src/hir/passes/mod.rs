pub mod ast_simplification;
pub mod counting;
pub mod lowering;
pub mod print;
pub mod typechecking;

use crate::ast::Program;
use crate::diagnostics::DiagnosticCollector;
use crate::pass::Pass;

/// Trait for HIR passes
pub trait HirPass {
    fn run(&mut self, program: &mut Program);
    fn diagnostics(&self) -> &DiagnosticCollector;
}

/// Blanket impl: any HirPass is also a Pass<Program>
impl<T: HirPass> Pass<Program> for T {
    fn run(&mut self, ir: &mut Program) {
        HirPass::run(self, ir)
    }
    fn diagnostics(&self) -> &DiagnosticCollector {
        HirPass::diagnostics(self)
    }
}
