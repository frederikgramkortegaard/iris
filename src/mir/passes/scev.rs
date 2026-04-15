use crate::diagnostics::DiagnosticCollector;
use crate::mir::analysis::dfg::DFGAnalysis;
use crate::mir::passes::MirPass;
use crate::mir::visitor::MirVisitor;
use crate::mir::Program;
use crate::mir::{Function, Instruction, Opcode, Operand, Reg};

pub struct MirSCEVPass {
    diagnostics: DiagnosticCollector,
}

impl Default for MirSCEVPass {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
enum SCEVExpr<'a> {
    AddRec {
        start: Box<SCEVExpr<'a>>,
        step: Box<SCEVExpr<'a>>,
    },
    BinOp {
        kind: Opcode,
        left: Box<SCEVExpr<'a>>,
        right: Box<SCEVExpr<'a>>,
    },
    Unknown(&'a Operand),
    Constant(&'a Operand),
    PhiSelf(&'a Operand),
}
impl MirSCEVPass {
    pub fn new() -> Self {
        MirSCEVPass {
            diagnostics: DiagnosticCollector::new(),
        }
    }

    fn build<'a>(
        &self,
        f: &'a Function,
        o: &'a Operand,
        dfg: &DFGAnalysis,
        phireg: Reg,
    ) -> SCEVExpr<'a> {
        match o {
            Operand::ImmI64(_) | Operand::ImmF64(_) | Operand::ImmBool(_) => SCEVExpr::Constant(o),

            Operand::Pair(_, arg) => self.build(f, arg, dfg, phireg),
            Operand::Reg(r) => {
                println!(",, {:?}, {:?}", r, phireg);
                if *r == phireg {
                    return SCEVExpr::PhiSelf(o);
                }

                let Some(inst) = dfg.get_instruction(f, *r) else {
                    println!(";; {:?} {:?}", o, r);
                    return SCEVExpr::Unknown(o);
                };

                match inst.op {
                    Opcode::Add | Opcode::Mul => {
                        let left = self.build(f, &inst.args[0], dfg, phireg);
                        let right = self.build(f, &inst.args[1], dfg, phireg);
                        SCEVExpr::BinOp {
                            kind: inst.op.clone(),
                            left: Box::new(left),
                            right: Box::new(right),
                        }
                    }
                    Opcode::Copy => self.build(f, &inst.args[0], dfg, phireg),
                    _ => SCEVExpr::Unknown(o),
                }
            }

            _ => SCEVExpr::Unknown(o),
        }
    }
}
impl MirVisitor for MirSCEVPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        // If no loops have been found for the function we can't run SCEV
        let Some(loops) = function.loops.as_ref() else {
            return;
        };

        if loops.is_empty() {
            return;
        }

        // Build DFG for O(1) definition lookups
        let dfg = DFGAnalysis::compute(function);

        // Let's find some BIVs
        for lop in loops {
            let block = function.block(lop.header);

            // The reason why we check Phis, is that induction variables will always at the top
            // level be a Phi node at this point, because they per definition are defined inside a
            // loop / in the loop header
            for phi in &block.phi_nodes {
                // We only handle the 'simple' case
                if phi.args.len() != 2 {
                    continue;
                }

                let (mut start, mut back) = (None, None);
                for arg in &phi.args {
                    let Operand::Pair(blockid, op) = arg else {
                        continue;
                    };
                    if lop.body.contains(blockid) {
                        back = Some(op);
                    } else {
                        start = Some(op);
                    }
                }

                let back_scev = self.build(function, back.unwrap(), &dfg, phi.dest);
                println!("{:?}", back_scev);

                let SCEVExpr::BinOp { kind, left, right } = &back_scev else {
                    continue;
                };

                if *kind != Opcode::Add {
                    continue;
                }

                let SCEVExpr::PhiSelf(o) = left.as_ref() else {
                    continue;
                };

                let SCEVExpr::Constant(_) = right.as_ref() else {
                    continue;
                };

                let start_scev = self.build(function, start.unwrap(), &dfg, phi.dest);
                println!("{:?}", start_scev);
                let SCEVExpr::Constant(_) = &start_scev else {
                    continue;
                };

                let addrec = SCEVExpr::AddRec {
                    start: Box::new(start_scev),
                    step: Box::new(back_scev),
                };
                println!("addrec: {:?}", addrec);
            }
        }

        println!("\n\n\n");
    }
}

impl MirPass for MirSCEVPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}
