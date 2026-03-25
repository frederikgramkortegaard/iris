use crate::diagnostics::DiagnosticCollector;
use crate::mir::cfg;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::Program;
use crate::mir::{BasicBlock, BlockId, Function, Instruction, Opcode, Operand, Reg, Terminator};
use std::collections::{HashMap, HashSet};

pub struct MirLoopPass {
    diagnostics: DiagnosticCollector,
    defs: HashMap<Reg, BlockId>,
}

pub type Header = BlockId;
pub type Latch = BlockId;

#[derive(Debug)]
pub struct Loop {
    header: BlockId,
    latches: Vec<BlockId>,
    body: HashSet<BlockId>,
    parent: Option<BlockId>,
    invariants: HashSet<Reg>,
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
            defs: HashMap::new(),
        }
    }

    fn find_back_edges(
        &self,
        function: &Function,
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
        // First compute all bodies
        let mut loop_data: Vec<(BlockId, Vec<BlockId>, HashSet<BlockId>)> = vec![];
        for (header, latches) in back_edges {
            let body = self.compute_body(*header, latches, predecessors);
            loop_data.push((*header, latches.clone(), body));
        }

        // Sort by body size descending (outer loops first)
        loop_data.sort_by(|a, b| b.2.len().cmp(&a.2.len()));

        let mut loops = vec![];
        for (header, latches, body) in loop_data {
            // Find parent: smallest existing loop that contains our header
            let parent = loops
                .iter()
                .filter(|l: &&Loop| l.body.contains(&header))
                .min_by_key(|l| l.body.len())
                .map(|l| l.header);

            loops.push(Loop {
                header,
                latches,
                body,
                parent,
                invariants: HashSet::new(),
            });
        }

        loops
    }

    fn compute_body(
        &self,
        header: BlockId,
        latches: &[BlockId],
        predecessors: &cfg::Predecessors,
    ) -> HashSet<BlockId> {
        let mut body = HashSet::new();
        body.insert(header);
        let mut stack: Vec<BlockId> = latches.to_vec();

        while let Some(node) = stack.pop() {
            if !body.contains(&node) {
                body.insert(node);
                if let Some(preds) = predecessors.get(&node) {
                    stack.extend(preds.iter().copied());
                }
            }
        }
        body
    }

    fn find_invariants(&mut self, function: &mut Function, lop: &Loop) -> HashSet<Reg> {
        let mut invariant: HashSet<Reg> = self
            .defs
            .iter()
            .filter(|(_, y)| !lop.body.contains(y))
            .map(|(reg, _)| *reg)
            .collect::<HashSet<Reg>>();

        // Admittedly a little less efficient to pre-collect the instructions, but it helps
        // clarity
        let instructions_in_loop: Vec<&Instruction> = lop
            .body
            .iter()
            .flat_map(|bid| function.block(*bid).instructions.iter())
            .collect();

        let mut converged = false;
        while !converged {
            converged = true;
            for inst in instructions_in_loop.iter().filter(|b| b.op != Opcode::Call) {
                if inst.args.iter().all(|b| match b {
                    Operand::Reg(r) => invariant.contains(r),
                    Operand::Label(..) | Operand::Pair(..) => false,
                    _ => true,
                }) {
                    converged = converged && !invariant.insert(inst.dest);
                }
            }
        }

        if crate::is_verbose() {
            println!("Found following invariants in {:?}: {:?}", lop, invariant);
        }
        invariant
    }
    fn licm(
        &mut self,
        function: &mut Function,
        loops: &Vec<Loop>,
        preds: &cfg::Predecessors,
        dominators: &cfg::DominatorSets,
    ) {
        for lop in loops {
            // Find hoistable instructions sorted by dominator tree order
            let mut sorted_body: Vec<BlockId> = lop.body.iter().copied().collect();
            sorted_body.sort_by_key(|b| dominators.get(b).map(|d| d.len()).unwrap_or(0));

            let inv = &lop.invariants;
            let mut to_hoist: Vec<(BlockId, Vec<usize>)> = vec![];
            for block in &sorted_body {
                let mut indices = vec![];
                for (i, inst) in function.block(*block).instructions.iter().enumerate() {
                    if inv.contains(&inst.dest)
                        && lop.body.contains(
                            self.defs
                                .get(&inst.dest)
                                .expect("instruction must have definition"),
                        )
                    {
                        indices.push(i);
                    }
                }

                if !indices.is_empty() {
                    to_hoist.push((*block, indices));
                }
            }

            if to_hoist.is_empty() {
                continue;
            }

            // Preheader
            let preheader = function.arena.alloc(BasicBlock {
                instructions: Vec::new(),
                terminator: Terminator::Br { target: lop.header },
                phi_nodes: Vec::new(),
                note: Some("preheader".to_string()),
            });

            let empty_preds = vec![];
            let outside_preds: Vec<BlockId> = preds
                .get(&lop.header)
                .unwrap_or(&empty_preds)
                .iter()
                .filter(|pred| !lop.body.contains(pred))
                .copied()
                .collect();

            // Redirect outside predecessors to preheader
            for outside in outside_preds {
                let block = function.block_mut(outside);
                match &mut block.terminator {
                    Terminator::Br { target } => {
                        if *target == lop.header {
                            *target = preheader;
                        }
                    }
                    Terminator::BrIf {
                        then_bb, else_bb, ..
                    } => {
                        if *then_bb == lop.header {
                            *then_bb = preheader;
                        }
                        if *else_bb == lop.header {
                            *else_bb = preheader;
                        }
                    }
                    _ => {}
                }
            }

            // add instruction <i> for i in indices to remove to preheader and
            // remove instruction <i> fro block.instructions
            for (id, indices) in to_hoist {
                // Collect in forward order
                let hoisted: Vec<Instruction> = indices
                    .iter()
                    .map(|&i| function.block(id).instructions[i].clone())
                    .collect();
                // Remove in reverse (keep indices valid)
                for i in indices.into_iter().rev() {
                    function.block_mut(id).instructions.remove(i);
                }
                // Append in forward order
                for inst in hoisted {
                    if crate::is_verbose() {
                        println!("Moving Instruction {:?} from {:?} to Preheader", inst, id);
                    }
                    function.block_mut(preheader).instructions.push(inst);
                }
            }
        }
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
    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        // Fill the defs map
        self.defs.clear();
        self.walk_function(function);
        for (reg, _) in &function.params {
            self.defs.insert(*reg, function.virtual_entry);
        }

        if crate::is_verbose() {
            println!("Function: '{}'", function.name);
        }
        let (predecessors, successors) = cfg::compute_cfg(function);
        let dominators = cfg::compute_dominators(function, &predecessors);

        let back_edges = self.find_back_edges(function, &successors, &dominators);
        let loops = self
            .find_loops(&back_edges, &predecessors)
            .into_iter()
            .map(|l| {
                // Find Invariants in each loop
                let invariants = self.find_invariants(function, &l);
                Loop {
                    header: l.header,
                    latches: l.latches,
                    body: l.body,
                    parent: l.parent,
                    invariants,
                }
            })
            .collect();

        self.licm(function, &loops, &predecessors, &dominators);
    }

    fn visit_basicblock(&mut self, block_id: BlockId, block: &mut BasicBlock) -> Self::Output {
        for phi in &block.phi_nodes {
            self.defs.insert(phi.dest, block_id);
        }

        for inst in &block.instructions {
            self.defs.insert(inst.dest, block_id);
        }
    }
}
impl MirPass for MirLoopPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
