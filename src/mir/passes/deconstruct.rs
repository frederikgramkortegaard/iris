use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::{BlockId, Function, Instruction, Opcode, Operand, Program, Reg, Type};
use std::collections::{BTreeMap, HashMap};

pub struct MirSSADeconstructionPass {
    diagnostics: DiagnosticCollector,
}

#[derive(Eq, PartialEq)]
enum Color {
    Grey,
    Black,
}
impl Default for MirSSADeconstructionPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirSSADeconstructionPass {
    pub fn new() -> Self {
        MirSSADeconstructionPass {
            diagnostics: DiagnosticCollector::new(),
        }
    }

    /// Collects all copies that need to be inserted into predecessor blocks to eliminate phi nodes.
    ///
    /// For each phi node `dst = phi(pred1: val1, pred2: val2, ...)`, each (predecessor, value) pair
    /// becomes a copy `dst <- val` that must execute at the end of that predecessor block.
    ///
    /// Returns a map of predecessor BlockId to a list of (dst, src, typ) triples -- the parallel
    /// copies that need to happen at the end of that block. The type is taken from the phi node
    /// and carried through so emitted Copy instructions have the correct type.
    ///
    /// These copies are parallel in the sense that they conceptually execute simultaneously.
    /// Before inserting them into the block, they must be sequentialized to handle
    /// ordering dependencies and cycles (the swap problem). See `sequentialize_copies`.
    fn collect_phi_copies(
        &self,
        function: &Function,
    ) -> BTreeMap<BlockId, Vec<(Reg, Operand, Type)>> {
        let mut copies = BTreeMap::new();

        for (_, block) in function.arena.iter() {
            for phi in &block.phi_nodes {
                // Each phi: dest = phi [pred1: val1], [pred2: val2], ...
                // phi.args is Vec<Operand::Pair(pred_block, value)>
                for arg in &phi.args {
                    if let Operand::Pair(pred_block, value) = arg {
                        copies.entry(*pred_block).or_insert_with(Vec::new).push((
                            phi.dest,
                            *value.clone(),
                            phi.typ,
                        ));
                    }
                }
            }
        }

        copies
    }

    fn sequentialize_copies(
        &self,
        function: &mut Function,
        phi_copies: &BTreeMap<BlockId, Vec<(Reg, Operand, Type)>>,
    ) {
        // DFS over the copy graph for one predecessor block.
        //
        // The copy list [(dst, src), ...] forms a directed graph where each edge goes
        // src -> dst, meaning "src must be read before dst is written."
        //
        // We traverse this graph with DFS coloring:
        //   - None  = not yet visited
        //   - Grey  = currently on the DFS stack (being explored)
        //   - Black = fully done, copy already emitted
        //
        // Non-cycle case: post-order emission naturally ensures every src is read
        // before its dst is written (we finish dependents before we finish the node).
        //
        // Cycle case (e.g. a <- b, b <- a):
        //   When we reach a Grey neighbor, we have a back edge -- a cycle.
        //   We break it by:
        //     1. Allocating a fresh tmp register
        //     2. Emitting: tmp <- node  (node is what the grey node wants to
        //        read, and it gets overwritten while we unwind)
        //     3. The grey node then reads tmp instead, in its normal
        //        post-order emission (tracked in the saved map)
        fn dfs(
            node: Reg,
            copies: &[(Reg, Operand, Type)],
            colors: &mut HashMap<Reg, Color>,
            saved: &mut HashMap<Reg, Reg>,
            result: &mut Vec<Instruction>,
            next_free_reg: &mut Reg,
        ) {
            colors.insert(node, Color::Grey);

            // Find all copies where src == node -- these are nodes that read from node
            // and must be fully emitted before we emit node's own copy.
            let neighbors: Vec<Reg> = copies
                .iter()
                .filter(|(_, src, _)| matches!(src, Operand::Reg(s) if *s == node))
                .map(|(dst, _, _)| *dst)
                .collect();

            for dst in neighbors {
                match colors.get(&dst) {
                    Some(Color::Grey) => {
                        // Back edge: dst is on the current DFS path, so we have a cycle.
                        // dst wants to read node, but node gets overwritten on the
                        // way back up. Save it now, dst reads the tmp later.
                        let tmp = *next_free_reg;
                        *next_free_reg += 1;

                        // Look up dst's type from the copy list
                        let dst_typ = copies
                            .iter()
                            .find(|(d, _, _)| *d == dst)
                            .map(|(_, _, t)| *t)
                            .unwrap_or(Type::I64);

                        // tmp <- node  (save node's current value before it gets clobbered)
                        result.push(Instruction {
                            dest: tmp,
                            op: Opcode::Copy,
                            typ: dst_typ,
                            args: vec![Operand::Reg(node)],
                        });

                        saved.insert(dst, tmp);
                    }
                    Some(Color::Black) => {
                        // Already fully processed, skip
                    }
                    None => {
                        // Unvisited, recurse into dst before emitting node
                        dfs(dst, copies, colors, saved, result, next_free_reg);
                    }
                }
            }

            // Post-order: all nodes that read from node are done, safe to emit
            // node's copy now. If the cycle case saved our source, read the tmp.
            if let Some((_, src, typ)) = copies.iter().find(|(dst, _, _)| *dst == node) {
                let src = match saved.get(&node) {
                    Some(&tmp) => Operand::Reg(tmp),
                    None => src.clone(),
                };
                result.push(Instruction {
                    dest: node,
                    op: Opcode::Copy,
                    typ: *typ,
                    args: vec![src],
                });
            }

            colors.insert(node, Color::Black);
        }

        for (pred_block_id, copies) in phi_copies {
            let mut colors: HashMap<Reg, Color> = HashMap::new();
            let mut saved: HashMap<Reg, Reg> = HashMap::new();
            let mut result: Vec<Instruction> = vec![];

            for (dst, _, _) in copies {
                if !colors.contains_key(dst) {
                    dfs(
                        *dst,
                        copies,
                        &mut colors,
                        &mut saved,
                        &mut result,
                        &mut function.next_free_reg,
                    );
                }
            }

            let block = function.arena.get_mut(*pred_block_id);
            block.instructions.extend(result);
        }
    }
}

impl MirVisitor for MirSSADeconstructionPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut Function) {
        let copies = self.collect_phi_copies(function);
        // Usually, you'd also do splitting of critical edges, but because
        // our IR is designed with structurec control flow (e.g. we create merge blocks proactively
        // instead of reactively) we don't actually need to do this

        // - Sequentialize parallel copies (handle swap problem)
        self.sequentialize_copies(function, &copies);
        for (_, block) in function.arena.iter_mut() {
            block.phi_nodes.clear();
        }
    }
}

impl MirPass for MirSSADeconstructionPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
