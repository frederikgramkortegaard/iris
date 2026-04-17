pub mod const_prop;
pub mod copy_prop;
pub mod dbe;
pub mod dce;
pub mod deconstruct;
pub mod gvn;
pub mod loops;
pub mod print;
pub mod reg_compact;
pub mod scev;
pub mod ssa;
pub mod tailcall;

use crate::diagnostics::DiagnosticCollector;
use crate::mir::Program;
use crate::pass::Pass;

/// Trait for MIR passes
pub trait MirPass {
    fn run(&mut self, program: &mut Program);
    fn diagnostics(&self) -> &DiagnosticCollector;
}

/// Blanket impl: any MirPass is also a Pass<Program>
impl<T: MirPass> Pass<Program> for T {
    fn run(&mut self, ir: &mut Program) {
        MirPass::run(self, ir)
    }
    fn diagnostics(&self) -> &DiagnosticCollector {
        MirPass::diagnostics(self)
    }
}
