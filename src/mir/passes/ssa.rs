use crate::diagnostics::DiagnosticCollector;
use crate::mir::cfg::CFGAnalysis;
use crate::mir::visitor::MirVisitor;
use crate::mir::{BlockId, MirFunction, MirProgram};
use std::collections::HashMap;
use std::collections::HashSet;

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

    /// Iterative data-flow method
    /// This generates a dominator set over the function by for each block,
    /// calculating the dominators of that block (blocks that we always end up
    /// going through if we want to go to this block)
    ///
    /// An example would be
    ///
    ///    A
    ///   / \
    ///  B   C
    ///   \ /
    ///    D
    ///
    /// Here, the dominator sets would be as such:
    ///     A: {A}
    ///     B: {A, B}
    ///     C: {A, C}
    ///     D: {A, D}
    ///
    /// Looking at node D, you see that only itself (dominator sets are self inclusive)
    /// and 'A' is in it's set. This is because you don't _need_ to go through B as you could
    /// go through C, and you don't need to go through C as you could go through B.
    ///
    /// We calculate this set by first:
    ///     1. Initialize every nodes dominator set to be ALL nodes
    ///     2. For every node, set dominators[node] = [Dom[p] intersect for p in
    ///        predeseccors[node]]
    ///     3. Once no change is observed after a calculation, we have stabilized and we are done.
    ///
    /// The intuition is, simpler to explain if we consider the types of nodes in the graph:
    ///     Nodes with no predecessors:
    ///     Nodes with a single predecessor:
    ///     Nodes with multiple predecessors:
    ///
    ///     If nodes have no predecessors (e.g. the entry block) their dominator set will only
    ///     contain themselves. {node}
    ///
    ///     If nodes only have a single predecessor, that nodes dominators will be the strict
    ///     superset {Dom[pred]} + {node}
    ///
    ///     If nodes have multiple predecessors, the logic can be thought of as:
    ///         The intersection of all of a nodes predecessors dominator sets, essentially maps
    ///         to the common ancestors list of node's predecessors.
    ///
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

    fn compute_idom(
        &mut self,
        function: &MirFunction,
        dominators: &HashMap<BlockId, HashSet<BlockId>>,
    ) -> HashMap<BlockId, BlockId> {
        let idom = HashMap::new();

        for (block_id, doms) in dominators {
            if block_id == &function.entry {
                continue;
            }

        }

        idom
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
        let mut dominators = self.compute_dominators(function, &cfg);
        for (b, s) in &dominators {
            println!("{:?}; {:?}", b, s);
        }
        let idom = self.compute_idom(function, &mut dominators);
    }
}
