use crate::diagnostics::DiagnosticCollector;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::{Function, Instruction, Opcode, Operand, Program, Reg};
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

    fn get_numeric(&self, o: &Operand) -> Option<f64> {
        match o {
            Operand::ImmI64(v) => Some(*v as f64),
            Operand::ImmF64(v) => Some(*v),
            Operand::ImmBool(v) => Some(*v as i64 as f64),
            _ => None,
        }
    }
    fn fold(&self, op: &Opcode, args: &[Operand]) -> Option<Operand> {
        if args.len() != 2 {
            return None;
        }

        let use_f64 =
            matches!(args[0], Operand::ImmF64(_)) || matches!(args[1], Operand::ImmF64(_));

        let wrap = |v: f64| {
            if use_f64 {
                Operand::ImmF64(v)
            } else {
                Operand::ImmI64(v as i64)
            }
        };

        let lhs = self.get_numeric(&args[0])?;
        let rhs = self.get_numeric(&args[1])?;

        match op {
            Opcode::Eq => Some(Operand::ImmBool(lhs == rhs)),
            Opcode::Ne => Some(Operand::ImmBool(lhs != rhs)),
            Opcode::Le => Some(Operand::ImmBool(lhs <= rhs)),
            Opcode::Lt => Some(Operand::ImmBool(lhs < rhs)),

            Opcode::Add => Some(wrap(lhs + rhs)),
            Opcode::Sub => Some(wrap(lhs - rhs)),
            Opcode::Div => Some(wrap(lhs / rhs)),
            Opcode::Mul => Some(wrap(lhs * rhs)),
            _ => None,
        }
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

    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
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
        if instruction.args.iter().all(|a| a.is_constant()) {
            if let Some(result) = self.fold(&instruction.op, &instruction.args) {
                instruction.op = Opcode::Copy;
                instruction.args = vec![result];
            }
        } else {
            for arg in &mut instruction.args {
                match arg {
                    Operand::Reg(r) => {
                        if let Some(constant) = self.constant_map.get(r) {
                            if crate::is_verbose() {
                                println!("Replacing register r{} with constant {:?}", r, constant);
                            }
                            *arg = constant.clone();
                        }
                    }
                    // Phi args are Pair(block, value) -- propagate into the inner value
                    Operand::Pair(_, op) => {
                        if let Operand::Reg(r) = op.as_ref() {
                            if let Some(constant) = self.constant_map.get(r) {
                                if crate::is_verbose() {
                                    println!(
                                        "Replacing (phi) register r{} with constant {:?}",
                                        r, constant
                                    );
                                }
                                **op = constant.clone();
                            }
                        }
                    }
                    _ => {}
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
            if crate::is_verbose() {
                println!(
                    "Register r{} is being assigned as a constant with value{:?}, adding it to `constant_map`",
                    instruction.dest,
                    &instruction.args[0]
                );
            }
        }
    }

    fn visit_terminator(&mut self, terminator: &mut crate::mir::Terminator) -> Self::Output {
        match terminator {
            crate::mir::Terminator::BrIf { cond, .. } => {
                if let Operand::Reg(r) = cond {
                    if let Some(constant) = self.constant_map.get(r) {
                        *cond = constant.clone();
                    }
                }
            }
            crate::mir::Terminator::Ret { value: Some(op) } => {
                if let Operand::Reg(r) = op {
                    if let Some(constant) = self.constant_map.get(r) {
                        *op = constant.clone();
                    }
                }
            }
            _ => {}
        }
    }
}

impl MirPass for MirConstPropPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
