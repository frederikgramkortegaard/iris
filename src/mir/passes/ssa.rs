use crate::diagnostics::DiagnosticCollector;
use crate::mir::cfg;
use crate::mir::visitor::MirVisitor;
use crate::mir::{MirFunction, MirProgram};

/// Converts MIR to SSA Form
pub struct MirSSAPass {
    diagnostics: DiagnosticCollector,
}

impl Default for MirSSAPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirSSAPass {
    pub fn new() -> Self {
        MirSSAPass {
            diagnostics: DiagnosticCollector::new(),
        }
    }

    pub fn convert(&mut self, program: &mut MirProgram) {
        self.visit_program(program);
    }
}

impl MirVisitor for MirSSAPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_program(&mut self, program: &mut MirProgram) -> Self::Output {
        self.walk_program(program);
    }

    fn visit_function(&mut self, function: &mut MirFunction) -> Self::Output {
        println!("Function: '{}'", function.name);
        let (predecessors, successors) = cfg::compute_cfg(function);
        let dominators = cfg::compute_dominators(function, &predecessors);

        println!("Dominators:");
        for (b, s) in &dominators {
            println!("{:?}; {:?}", b, s);
        }
        let dtree = cfg::compute_dominator_tree(function, &dominators, &successors);
        println!("Dominator Tree (map)\n{:?}",dtree);

        let dfront = cfg::compute_dominator_frontier(&dtree, &predecessors);
        println!("Dominator Frontier (set)\n{:?}", dfront);

    }
}
