use crate::diagnostics::DiagnosticCollector;
use crate::mir::cfg::CFGAnalysis;
use crate::mir::visitor::MirVisitor;
use crate::mir::{BasicBlock, BlockId, Instruction, MirFunction, MirProgram, Operand, Terminator};
use std::collections::HashMap;
use std::collections::HashSet;

/// Converts MIR to SSA Form
pub struct MirSSAPass {
    diagnostics: DiagnosticCollector,
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

    /// Iterative data-flow method
    pub fn compute_dominators(
        &mut self,
        function: &MirFunction,
        cfg: &CFGAnalysis,
    ) -> HashMap<BlockId, HashSet<BlockId>> {
        let mut dom: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();
        let all_blocks: Vec<BlockId> = function.arena.iter().map(|(a, _)| a).collect();

        for &node in &all_blocks {
            if node == function.entry {
                dom.insert(node, HashSet::from([function.entry]));
            } else {
                dom.insert(node, HashSet::from_iter(all_blocks.clone()));
            }
        }

        loop {
            let mut changed = false;
            for &node in &all_blocks {
                if node == function.entry {
                    continue;
                }
                let preds = cfg.predecessors.get(&node).unwrap();

                if preds.is_empty() {
                    continue;
                }

                let mut inter: HashSet<BlockId> = dom.get(&preds[0]).unwrap().clone();
                for &p in &preds[1..] {
                    inter.retain(|x| dom.get(&p).unwrap().contains(x));
                }

                inter.insert(node);

                if inter != dom[&node] {
                    changed = true;
                    dom.insert(node, inter);
                }
            }

            if !changed {
                break;
            }
        }
        dom
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
        let cfg = CFGAnalysis::new(function);
        let dominators = self.compute_dominators(function, &cfg);
        for (b, s) in dominators {
            println!("{:?}; {:?}", b, s);
        }
    }
}
