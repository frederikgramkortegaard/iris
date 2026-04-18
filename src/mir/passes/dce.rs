use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::Program;
use crate::mir::{BasicBlock, BlockId, Function, Opcode, Operand, Reg, Terminator, Type};
use std::collections::{HashMap, HashSet};

type InstructionIndex = usize;

type PhiIndex = usize;

/// Dead Code Elimination pass
pub struct MirDCEPass {
    diagnostics: DiagnosticCollector,
    defmap: HashMap<Reg, (BlockId, InstructionIndex)>,
    phi_defmap: HashMap<Reg, (BlockId, PhiIndex)>,
    live: HashSet<Reg>,
    worklist: Vec<Reg>,
}

impl Default for MirDCEPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirDCEPass {
    pub fn new() -> Self {
        MirDCEPass {
            diagnostics: DiagnosticCollector::new(),
            defmap: HashMap::new(),
            phi_defmap: HashMap::new(),
            live: HashSet::new(),
            worklist: vec![],
        }
    }

    fn has_side_effects(&self, op: &Opcode) -> bool {
        matches!(op, Opcode::Call)
    }

    fn propagate_worklist(&mut self, function: &Function) {
        while let Some(reg) = self.worklist.pop() {
            // Check regular instructions
            if let Some((block_id, idx)) = self.defmap.get(&reg) {
                let inst = &function.arena.get(*block_id).instructions[*idx];
                for arg in &inst.args {
                    if let Operand::Reg(r) = arg {
                        if self.live.insert(*r) {
                            self.worklist.push(*r);
                        }
                    }
                }
            }
            // Check phi nodes
            if let Some((block_id, idx)) = self.phi_defmap.get(&reg) {
                let phi = &function.arena.get(*block_id).phi_nodes[*idx];
                for arg in &phi.args {
                    if let Operand::Pair(_, inner) = arg {
                        if let Operand::Reg(r) = inner.as_ref() {
                            if self.live.insert(*r) {
                                self.worklist.push(*r);
                            }
                        }
                    }
                }
            }
        }
    }
    fn sweep(&self, function: &mut Function) {
        for (_, block) in function.arena.iter_mut() {
            block.instructions.retain(|inst| {
                let keep = self.live.contains(&inst.dest);
                if !keep && crate::is_verbose() {
                    println!("Removing Instruction {:?} from block", inst);
                }
                keep
            });
            block.phi_nodes.retain(|phi| {
                let keep = self.live.contains(&phi.dest);
                if !keep && crate::is_verbose() {
                    println!("Removing dead Phi {:?} from block", phi);
                }
                keep
            });
        }
    }
}

// The visitor is used to mark liveness and build the defmap
impl MirVisitor for MirDCEPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        self.defmap.clear();
        self.phi_defmap.clear();
        self.live.clear();
        self.worklist.clear();

        self.walk_function(function);
        if crate::is_verbose() {
            println!(
                "Function: {}:\ndefmap: {:?}\nlive: {:?}\nworklist: {:?}\n\n",
                function.name, self.defmap, self.live, self.worklist
            );
        }

        self.propagate_worklist(function);

        self.sweep(function);
        if crate::is_verbose() {
            println!(
                "Function: {}:\ndefmap: {:?}\nlive: {:?}\nworklist: {:?}\n\n",
                function.name, self.defmap, self.live, self.worklist
            );
        }
    }

    fn visit_basicblock(&mut self, block_id: BlockId, block: &mut BasicBlock) -> Self::Output {
        // Track phi definitions
        for (i, phi) in block.phi_nodes.iter().enumerate() {
            self.phi_defmap.insert(phi.dest, (block_id, i));
        }

        for (i, instruction) in block.instructions.iter().enumerate() {
            self.defmap.insert(instruction.dest, (block_id, i));

            // If the instruction has (or could have) side effects, we assume the dest is live,
            // because the defining line itself can't be known to be safe to delete at this point
            if self.has_side_effects(&instruction.op) {
                self.live.insert(instruction.dest);

                // Because of this, all of the arguments must also be live
                for arg in &instruction.args {
                    if let Operand::Reg(r) = arg {
                        // If the value does NOT exist in live, we add it to the worklist. If it
                        // didnt exist there, it means its the first time we're marking R as
                        // live and thus we should process it
                        if self.live.insert(*r) {
                            self.worklist.push(*r);
                        }
                    }
                }
            }
        }

        self.walk_basicblock(block);
    }

    fn visit_terminator(&mut self, term: &mut Terminator) -> Self::Output {
        match &term {
            Terminator::Ret {
                value: Some(Operand::Reg(r)),
            } if self.live.insert(*r) => {
                self.worklist.push(*r);
            }
            Terminator::BrIf {
                cond: Operand::Reg(r),
                ..
            } if self.live.insert(*r) => {
                self.worklist.push(*r);
            }
            _ => {}
        }
    }

    fn visit_param(&mut self, reg: Reg, _type: Type) -> Self::Output {
        // Parameters are special, they are ALWAYS considered live @TODO : Maybe it would be
        // possible to do some weird optimization here
        self.live.insert(reg);
    }
}
impl MirPass for MirDCEPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
