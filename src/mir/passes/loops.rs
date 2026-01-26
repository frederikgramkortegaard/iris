use crate::diagnostics::DiagnosticCollector;
use crate::mir::cfg;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::MirProgram;
use crate::mir::{BlockId, MirFunction, Opcode, Operand, Reg};
use std::collections::{HashMap, HashSet};

pub struct MirLoopPass {
    diagnostics: DiagnosticCollector,
}

pub type Header = BlockId;
pub type Latch = BlockId;

pub struct Loop {
    header: BlockId,
    latches: Vec<BlockId>,
    body: HashSet<BlockId>,
    parent: Option<BlockId>,
}

impl Default for MirLoopPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirLoopPass {
    pub fn new() -> Self {
        MirLoopPass {
            diagnostics: DiagnosticCollector::new(),
        }
    }

    fn find_back_edges(
        &self,
        function: &MirFunction,
        successors: &cfg::Successors,
        dominators: &cfg::DominatorSets,
    ) -> HashMap<Header, Vec<Latch>> {
        let empty: HashSet<BlockId> = HashSet::new();

        // Build map of back edges, grouped by header
        let mut back_edges: HashMap<Header, Vec<Latch>> = HashMap::new();
        for (id, _) in function.arena.iter() {
            for succ in successors.get(&id).unwrap_or(&vec![]) {
                if dominators.get(&id).unwrap_or(&empty).contains(succ) {
                    back_edges.entry(*succ).or_default().push(id);
                }
            }
        }

        back_edges
    }

    fn find_loops(
        &self,
        back_edges: &HashMap<Header, Vec<Latch>>,
        predecessors: &cfg::Predecessors,
    ) -> Vec<Loop> {
        let mut loops = vec![];
        for (header, latches) in back_edges {
            let mut body: HashSet<BlockId> = HashSet::new();
            body.insert(*header);

            let mut stack: Vec<BlockId> = latches.clone();

            while let Some(node) = stack.pop() {
                if !body.contains(&node) {
                    body.insert(node);
                    if let Some(preds) = predecessors.get(&node) {
                        stack.extend(preds.iter().copied());
                    }
                }
            }

            loops.push(Loop {
                header: *header,
                latches: latches.clone(),
                body,
                parent: None,
            });
        }

        loops
    }
}

impl MirVisitor for MirLoopPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }
    fn visit_function(&mut self, function: &mut MirFunction) -> Self::Output {
        println!("Function: '{}'", function.name);
        let (predecessors, successors) = cfg::compute_cfg(function);
        let dominators = cfg::compute_dominators(function, &predecessors);

        let back_edges = self.find_back_edges(function, &successors, &dominators);
        let loops = self.find_loops(&back_edges, &predecessors);

        for l in &loops {
            println!(
                "Loop: header={:?}, latches={:?}, body={:?}",
                l.header, l.latches, l.body
            );
        }
    }
}
impl MirPass for MirLoopPass {
    fn run(&mut self, program: &mut MirProgram) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
