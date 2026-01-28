pub mod const_prop;
pub mod copy_prop;
pub mod dce;
pub mod gvn;
pub mod loops;
pub mod print;
pub mod ssa;
pub mod tailcall;

use crate::diagnostics::DiagnosticCollector;
use crate::mir::MirProgram;
use crate::pass::Pass;

/// Trait for MIR passes
pub trait MirPass {
    fn run(&mut self, program: &mut MirProgram);
    fn diagnostics(&self) -> &DiagnosticCollector;
}

/// Blanket impl: any MirPass is also a Pass<MirProgram>
impl<T: MirPass> Pass<MirProgram> for T {
    fn run(&mut self, ir: &mut MirProgram) {
        MirPass::run(self, ir)
    }
    fn diagnostics(&self) -> &DiagnosticCollector {
        MirPass::diagnostics(self)
    }
}
