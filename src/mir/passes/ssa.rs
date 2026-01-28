use crate::diagnostics::DiagnosticCollector;
use crate::mir::cfg;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::{
    BlockId, Instruction, Function, Program, Type, Opcode, Operand, Reg, Terminator,
};
use std::collections::{HashMap, HashSet};

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

    pub fn convert(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn rename_variables_to_ssa(
        function: &mut Function,
        dtree: &cfg::DominatorTree,
        successors: &cfg::Successors,
        original_registers: &HashMap<BlockId, Vec<Reg>>,
    ) {
        // Invert the Dominator Tree (Map) such that instead of Child -> Parent, it's Parent ->
        // Child
        let mut inverted_dominator_tree: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        for (&child, &parent) in dtree {
            inverted_dominator_tree
                .entry(parent)
                .or_default()
                .push(child);
        }

        let mut counter: HashMap<Reg, usize> = HashMap::new();
        let mut stack: HashMap<Reg, Vec<usize>> = HashMap::new();

        for (param_reg, _type) in &function.params {
            stack.entry(*param_reg).or_default().push(*param_reg);
        }

        // Inner method for actually doing the recursive renaming
        fn rename(
            counter: &mut HashMap<Reg, usize>,
            stack: &mut HashMap<Reg, Vec<usize>>,
            function: &mut Function,
            entry: BlockId,
            children: &HashMap<BlockId, Vec<BlockId>>,
            successors: &cfg::Successors,
            original_registers: &HashMap<BlockId, Vec<Reg>>,
        ) {
            // Track what we push so we can pop when leaving this block
            let mut pushed: Vec<Reg> = vec![];

            let block = function.arena.get_mut(entry);

            // Update counter and stack with definitions from phi nodes
            for phi_node in &mut block.phi_nodes {
                let name = phi_node.dest;
                *counter.entry(phi_node.dest).or_insert(1) += 1;
                let fresh = function.next_free_reg;
                stack.entry(name).or_default().push(fresh);
                pushed.push(name);
                function.next_free_reg += 1;
                phi_node.dest = fresh;
            }

            // Replace Instruction Uses and Destinations
            for inst in &mut block.instructions {
                for arg in &mut inst.args {
                    if let Operand::Reg(r) = arg {
                        if let Some(current) = stack.get(r).and_then(|v| v.last()) {
                            *r = *current;
                        }
                    }
                }

                // Create a new register (var name) for the destination
                let original_dest = inst.dest;
                let fresh = function.next_free_reg;
                function.next_free_reg += 1;
                stack.entry(original_dest).or_default().push(fresh);
                pushed.push(original_dest);
                inst.dest = fresh;

                // Handle terminator uses, e.g. return values or branch conditions
                // If its a call, we have to go through every argument in the call and update it
                if let Opcode::Call = inst.op {
                    for arg in &mut inst.args {
                        if let Operand::Reg(r) = arg {
                            if let Some(current) = stack.get(r).and_then(|v| v.last()) {
                                *r = *current;
                            }
                        }
                    }
                }
            }

            let op: Option<&mut Operand> = match &mut block.terminator {
                Terminator::Ret { value: Some(op) } => Some(op),

                Terminator::BrIf { cond, .. } => Some(cond),
                _ => None,
            };

            if let Some(Operand::Reg(r)) = op {
                if let Some(current) = stack.get(r).and_then(|v| v.last()) {
                    *r = *current;
                }
            }
            // Fill in phi nodes in successor blocks
            let empty: Vec<Reg> = vec![];
            for &succ in successors.get(&entry).into_iter().flatten() {
                let original_regs = original_registers.get(&succ).unwrap_or(&empty);
                for (i, phi_node) in &mut function
                    .arena
                    .get_mut(succ)
                    .phi_nodes
                    .iter_mut()
                    .enumerate()
                {
                    if let Some(&original_reg) = original_regs.get(i) {
                        if let Some(&current) = stack.get(&original_reg).and_then(|v| v.last()) {
                            phi_node
                                .args
                                .push(Operand::Pair(entry, Box::new(Operand::Reg(current))));
                        }
                    }
                }
            }

            // Recurse to dominator tree children
            for &child in children.get(&entry).into_iter().flatten() {
                rename(
                    counter,
                    stack,
                    function,
                    child,
                    children,
                    successors,
                    original_registers,
                );
            }

            // Pop everything we pushed in this block
            for reg in pushed {
                stack.get_mut(&reg).unwrap().pop();
            }
        }

        let entry = function.virtual_entry;
        rename(
            &mut counter,
            &mut stack,
            function,
            entry,
            &inverted_dominator_tree,
            successors,
            original_registers,
        );
    }

    fn insert_phi_nodes(
        function: &mut Function,
        dfront: &cfg::DominatorFrontier,
    ) -> HashMap<BlockId, Vec<Reg>> {
        let empty = HashSet::new();

        let mut original_registers: HashMap<BlockId, Vec<Reg>> = HashMap::new();

        for (reg, definers) in function.definitions.clone().iter() {
            // Single definition, no phi needed
            if definers.len() < 2 {
                continue;
            }

            let mut has_phi: HashSet<BlockId> = HashSet::new();
            let mut worklist: Vec<BlockId> = definers.iter().copied().collect();

            while let Some(block) = worklist.pop() {
                for &frontier in dfront.get(&block).unwrap_or(&empty) {
                    if !has_phi.contains(&frontier) {
                        original_registers.entry(frontier).or_default().push(*reg);

                        function
                            .arena
                            .get_mut(frontier)
                            .phi_nodes
                            .push(Instruction {
                                dest: *reg,
                                op: Opcode::Phi,
                                typ: Type::Void,
                                args: vec![],
                            });

                        has_phi.insert(frontier);
                        worklist.push(frontier);
                    }
                }
            }
        }

        original_registers
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

    fn visit_program(&mut self, program: &mut Program) -> Self::Output {
        self.walk_program(program);
    }

    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        for (param_reg, _) in &function.params {
            function
                .definitions
                .entry(*param_reg)
                .or_default()
                .insert(function.virtual_entry);
        }

        println!("Function: '{}'", function.name);
        let (predecessors, successors) = cfg::compute_cfg(function);
        let dominators = cfg::compute_dominators(function, &predecessors);

        println!("Dominators:");
        for (b, s) in &dominators {
            println!("{:?}; {:?}", b, s);
        }
        let dtree = cfg::compute_dominator_tree(function, &dominators, &successors);
        println!("Dominator Tree (map)\n{:?}", dtree);

        let dfront = cfg::compute_dominator_frontier(&dtree, &predecessors);
        println!("Dominator Frontier (set)\n{:?}", dfront);

        let original_registers = Self::insert_phi_nodes(function, &dfront);
        Self::rename_variables_to_ssa(function, &dtree, &successors, &original_registers);
    }
}

impl MirPass for MirSSAPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
