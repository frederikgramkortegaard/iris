use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::{Function, Operand, Program, Reg, Terminator};
use std::collections::{BTreeSet, HashMap};

pub struct RegCompactPass {
    diagnostics: DiagnosticCollector,
}

impl Default for RegCompactPass {
    fn default() -> Self {
        Self::new()
    }
}

impl RegCompactPass {
    pub fn new() -> Self {
        RegCompactPass {
            diagnostics: DiagnosticCollector::new(),
        }
    }

    fn compact_function(&self, function: &mut Function) {
        let mut used = BTreeSet::new();

        for &(reg, _) in &function.params {
            used.insert(reg);
        }

        for (_, block) in function.arena.iter() {
            for inst in &block.instructions {
                used.insert(inst.dest);
                for arg in &inst.args {
                    Self::collect_operand_regs(arg, &mut used);
                }
            }
            match &block.terminator {
                Terminator::Ret { value: Some(op) } => Self::collect_operand_regs(op, &mut used),
                Terminator::BrIf { cond, .. } => Self::collect_operand_regs(cond, &mut used),
                _ => {}
            }
        }

        // Build old -> new mapping (BTreeSet gives sorted order)
        let mapping: HashMap<Reg, Reg> = used
            .iter()
            .enumerate()
            .map(|(new, &old)| (old, new))
            .collect();

        // Rewrite params
        for param in &mut function.params {
            param.0 = mapping[&param.0];
        }

        // Rewrite all blocks
        for (_, block) in function.arena.iter_mut() {
            for inst in &mut block.instructions {
                inst.dest = mapping[&inst.dest];
                for arg in &mut inst.args {
                    Self::rewrite_operand(arg, &mapping);
                }
            }
            match &mut block.terminator {
                Terminator::Ret { value: Some(op) } => Self::rewrite_operand(op, &mapping),
                Terminator::BrIf { cond, .. } => Self::rewrite_operand(cond, &mapping),
                _ => {}
            }
        }

        function.next_free_reg = used.len();
    }

    fn collect_operand_regs(op: &Operand, used: &mut BTreeSet<Reg>) {
        match op {
            Operand::Reg(r) => {
                used.insert(*r);
            }
            Operand::Pair(_, inner) => Self::collect_operand_regs(inner, used),
            _ => {}
        }
    }

    fn rewrite_operand(op: &mut Operand, mapping: &HashMap<Reg, Reg>) {
        match op {
            Operand::Reg(r) => *r = mapping[r],
            Operand::Pair(_, inner) => Self::rewrite_operand(inner, mapping),
            _ => {}
        }
    }
}

impl MirPass for RegCompactPass {
    fn run(&mut self, program: &mut Program) {
        for function in &mut program.functions {
            self.compact_function(function);
        }
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
