use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::{Instruction, MirFunction, MirProgram, Opcode, Operand, Reg};
use std::collections::HashMap;

pub struct MirConstPropPass {
    diagnostics: DiagnosticCollector,
    constant_map: HashMap<Reg, Operand>,
}

impl Default for MirConstPropPass {
    fn default() -> Self {
        Self::new()
    }
}

impl MirConstPropPass {
    pub fn new() -> Self {
        MirConstPropPass {
            diagnostics: DiagnosticCollector::new(),
            constant_map: HashMap::new(),
        }
    }

    fn is_rhs_constant(&self, op: &Operand) -> bool {
        !matches!(op, Operand::Reg(_) | Operand::Pair(..) | Operand::Label(_))
    }
}

impl MirVisitor for MirConstPropPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut MirFunction) -> Self::Output {
        self.walk_function(function);
        self.constant_map.clear();
    }

    //@NOTE : This is just Constant Propagation, NOT Constant folding
    // it does not even remove the redundant instructions, as we're leaving that to the DCE>
    // In the future, we will implement an `SSAConstantFolder` module and use that in here,
    // and then instead of checking if the rhs is constant, we can do this:
    //
    // fn fold_constants(operands: &Vec<Operand>) -> Option<Operand> {
    //      if operands.len() < 2 {
    //          ...
    //      }
    //
    //      ... here we would slide a window over the operands list, continually replace the LHS
    //      with a newly created operand, if maybe at operand 3 we can no longer constant fold,
    //      it is not a constant rhs in totality and we just return None
    //      otherwise we can return the newly constructed Operand which will
    //      replace the entire operand list in the instruction
    // }
    fn visit_instruction(&mut self, instruction: &mut Instruction) -> Self::Output {
        for arg in &mut instruction.args {
            if let Operand::Reg(r) = arg {
                if let Some(constant) = self.constant_map.get(r) {
                    println!("Replacing register r{} with constant {:?}", r, constant);
                    *arg = constant.clone();
                }
            }
        }

        if matches!(instruction.op, Opcode::Copy)
            && instruction.args.len() == 1
            && self.is_rhs_constant(&instruction.args[0])
        {
            self.constant_map
                .entry(instruction.dest)
                .or_insert(instruction.args[0].clone());
            println!(
                "Register r{} is being assigned as a constant with value{:?}, adding it to `constant_map`",
                instruction.dest,
                instruction.args[0].clone()
            );
        }
    }
}

impl MirPass for MirConstPropPass {
    fn run(&mut self, program: &mut MirProgram) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
