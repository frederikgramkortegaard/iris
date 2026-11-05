use crate::diagnostics::DiagnosticCollector;
use crate::mir::analysis::{callgraph, loops as loop_analysis, recursion};
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::{
    BasicBlock, BlockId, FuncId, Function, Instruction, Opcode, Operand, Program, Reg, Terminator,
};
use log::debug;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub struct MirInliningPass {
    diagnostics: DiagnosticCollector,
}

impl Default for MirInliningPass {
    fn default() -> Self {
        Self::new()
    }
}

/// Straight-line callees are inlined up to this many instructions.
const INLINE_MAX_INSTRUCTIONS: usize = 50;

/// A callee that contains a loop amortizes its own call overhead, inlining
/// it buys nothing and bloats the caller's hot path (mandel runs ~12%
/// slower with cell inlined into the pixel loop). Loop-free callees are
/// worth it while they stay small.
fn should_inline(callee: &Function) -> bool {
    if !loop_analysis::analyze(callee).is_empty() {
        return false;
    }
    let size: usize = callee.arena.iter().map(|(_, b)| b.instructions.len()).sum();
    size <= INLINE_MAX_INSTRUCTIONS
}

impl MirInliningPass {
    pub fn new() -> Self {
        MirInliningPass {
            diagnostics: DiagnosticCollector::new(),
        }
    }
}

impl MirVisitor for MirInliningPass {
    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_program(&mut self, program: &mut Program) {
        let mut callgraph = callgraph::compute_callgraph(&program.functions);
        let recursive: BTreeSet<FuncId> = callgraph
            .keys()
            .filter(|&&fid| recursion::can_function_reach_itself(fid, &callgraph))
            .copied()
            .collect();

        for children in callgraph.values_mut() {
            children.retain(|callee| !recursive.contains(callee));
        }

        // one should_inline verdict per callee, decided on first encounter
        let mut inline_ok: HashMap<FuncId, bool> = HashMap::new();
        let func_name_to_id: HashMap<String, FuncId> = program
            .functions
            .iter()
            .map(|(id, f)| (f.name.clone(), id))
            .collect();
        let mut processed: HashSet<FuncId> = HashSet::new();
        while processed.len() < callgraph.len() {
            let ready: Vec<FuncId> = callgraph
                .iter()
                .filter(|(k, v)| !processed.contains(k) && v.iter().all(|c| processed.contains(c)))
                .map(|(k, _)| *k)
                .collect();

            // post-stripping the graph is a DAG, so there is always a ready
            // function left. empty means the recursion marking leaked a cycle
            assert!(
                !ready.is_empty(),
                "no ready functions, cycle leaked past recursion marking"
            );

            for fid in ready {
                let sites: Vec<(BlockId, usize, FuncId)> = {
                    let caller = program.functions.get(fid);
                    let mut sites = Vec::new();
                    for (bid, block) in caller.arena.iter() {
                        for (idx, inst) in block.instructions.iter().enumerate() {
                            let name = match (&inst.op, inst.args.first()) {
                                (Opcode::Call, Some(Operand::Label(name))) => name,
                                _ => continue,
                            };
                            let Some(&callee_id) = func_name_to_id.get(name) else {
                                debug!("call to unknown function '{}', not inlining", name);
                                continue;
                            };
                            if recursive.contains(&callee_id) {
                                continue;
                            }
                            let worth_it = *inline_ok
                                .entry(callee_id)
                                .or_insert_with(|| should_inline(program.functions.get(callee_id)));
                            if !worth_it {
                                debug!("not inlining '{}': has a loop or too big", name);
                                continue;
                            }

                            sites.push((bid, idx, callee_id));
                        }
                    }
                    sites
                };

                for (bid, idx, callee_id) in sites.into_iter().rev() {
                    let (callee_blocks, callee_params, callee_defs, callee_entry) = {
                        let callee = program.functions.get(callee_id);
                        let blocks: Vec<_> = callee
                            .arena
                            .iter()
                            .filter(|(b, _)| *b != callee.virtual_entry)
                            .map(|(b, block)| (b, block.clone()))
                            .collect();
                        (
                            blocks,
                            callee.params.clone(),
                            callee.definitions.clone(),
                            callee.entry,
                        )
                    };

                    let caller: &mut Function = program.functions.get_mut(fid);

                    let mut bim: BTreeMap<BlockId, BlockId> = BTreeMap::new();
                    for (old_bid, block) in callee_blocks {
                        bim.insert(old_bid, caller.arena.alloc(block));
                    }

                    // Update terminators
                    for &new_bid in bim.values() {
                        caller
                            .arena
                            .get_mut(new_bid)
                            .terminator
                            .map_targets(|t| bim.get(&t).copied().unwrap_or(t));
                    }

                    let mut used = BTreeSet::new();
                    for &(reg, _) in &callee_params {
                        used.insert(reg);
                    }
                    for &new_bid in bim.values() {
                        let block = caller.arena.get(new_bid);
                        for inst in &block.instructions {
                            used.insert(inst.dest);
                            for arg in &inst.args {
                                arg.for_each_reg(&mut |r| {
                                    used.insert(r);
                                });
                            }
                        }
                        for op in block.terminator.operands() {
                            op.for_each_reg(&mut |r| {
                                used.insert(r);
                            });
                        }
                    }

                    // Build old -> new mapping
                    let mapping: HashMap<Reg, Reg> = used
                        .iter()
                        .map(|&old| (old, caller.reserve_next_free_reg()))
                        .collect();

                    // Rewrite all blocks
                    for &new_bid in bim.values() {
                        let block = caller.arena.get_mut(new_bid);
                        for inst in &mut block.instructions {
                            inst.dest = mapping[&inst.dest];
                            for arg in &mut inst.args {
                                arg.map_regs(&mut |r| mapping[&r]);
                            }
                        }
                        for op in block.terminator.operands_mut() {
                            op.map_regs(&mut |r| mapping[&r]);
                        }
                    }

                    // Update definitions
                    for (reg, blocks) in callee_defs.iter() {
                        caller.definitions.insert(
                            mapping[reg],
                            blocks.iter().filter_map(|b| bim.get(b).copied()).collect(),
                        );
                    }

                    let inline_entry = bim[&callee_entry];

                    let (call, tail, old_terminator) = {
                        let block = caller.arena.get_mut(bid);
                        let tail = block.instructions.split_off(idx + 1);
                        let call = block
                            .instructions
                            .pop()
                            .expect("site index must point at the call");
                        for (&(preg, ptyp), arg) in callee_params.iter().zip(&call.args[1..]) {
                            block.instructions.push(Instruction {
                                dest: mapping[&preg],
                                op: Opcode::Copy,
                                typ: ptyp,
                                args: vec![arg.clone()],
                            });
                        }
                        let old_terminator = std::mem::replace(
                            &mut block.terminator,
                            Terminator::Br {
                                target: inline_entry,
                            },
                        );
                        (call, tail, old_terminator)
                    };

                    for &(preg, _) in &callee_params {
                        caller
                            .definitions
                            .entry(mapping[&preg])
                            .or_default()
                            .insert(bid);
                    }

                    let tail_defs: Vec<Reg> = tail.iter().map(|i| i.dest).collect();
                    let continuation = caller.arena.alloc(BasicBlock {
                        instructions: tail,
                        terminator: old_terminator,
                        phi_nodes: Vec::new(),
                        note: Some("continuation block after inlining".to_string()),
                    });
                    for dest in tail_defs {
                        caller
                            .definitions
                            .entry(dest)
                            .or_default()
                            .insert(continuation);
                    }

                    for &new_bid in bim.values() {
                        let block = caller.arena.get_mut(new_bid);
                        let Terminator::Ret { value } = &block.terminator else {
                            continue;
                        };
                        if let Some(v) = value.clone() {
                            block.instructions.push(Instruction {
                                dest: call.dest,
                                op: Opcode::Copy,
                                typ: call.typ,
                                args: vec![v],
                            });
                            caller
                                .definitions
                                .entry(call.dest)
                                .or_default()
                                .insert(new_bid);
                        }
                        block.terminator = Terminator::Br {
                            target: continuation,
                        };
                    }
                }

                processed.insert(fid);
            }
        }
    }
}

impl MirPass for MirInliningPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
