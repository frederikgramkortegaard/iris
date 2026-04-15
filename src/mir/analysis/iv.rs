use crate::mir::analysis::dfg::DFGAnalysis;
use crate::mir::analysis::loops::Loop;
use crate::mir::{Function, Instruction, Opcode, Operand, Reg};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum SCEVExpr<'a> {
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

fn build<'a>(
    f: &'a Function,
    o: &'a Operand,
    dfg: &DFGAnalysis,
    phireg: Reg,
) -> SCEVExpr<'a> {
    match o {
        Operand::ImmI64(_) | Operand::ImmF64(_) | Operand::ImmBool(_) => SCEVExpr::Constant(o),
        Operand::Pair(_, arg) => build(f, arg, dfg, phireg),
        Operand::Reg(r) => {
            if *r == phireg {
                return SCEVExpr::PhiSelf(o);
            }

            let Some(inst) = dfg.get_instruction(f, *r) else {
                return SCEVExpr::Unknown(o);
            };

            match inst.op {
                Opcode::Add | Opcode::Mul => {
                    let left = build(f, &inst.args[0], dfg, phireg);
                    let right = build(f, &inst.args[1], dfg, phireg);
                    SCEVExpr::BinOp {
                        kind: inst.op.clone(),
                        left: Box::new(left),
                        right: Box::new(right),
                    }
                }
                Opcode::Copy => build(f, &inst.args[0], dfg, phireg),
                _ => SCEVExpr::Unknown(o),
            }
        }
        _ => SCEVExpr::Unknown(o),
    }
}

fn try_build_addrec<'a>(
    f: &'a Function,
    dfg: &DFGAnalysis,
    phi: &'a Instruction,
    lop: &Loop,
) -> Option<SCEVExpr<'a>> {

    // Only Three-Address Phi Nodes
    if phi.args.len() != 2 {
        return None;
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

    let start = start?;
    let back = back?;

    let back_scev = build(f, back, dfg, phi.dest);

    let SCEVExpr::BinOp { kind, left, right } = &back_scev else {
        return None;
    };

    if *kind != Opcode::Add {
        return None;
    }

    let SCEVExpr::PhiSelf(_) = left.as_ref() else {
        return None;
    };

    let SCEVExpr::Constant(_) = right.as_ref() else {
        return None;
    };

    let start_scev = build(f, start, dfg, phi.dest);
    let SCEVExpr::Constant(_) = &start_scev else {
        return None;
    };

    Some(SCEVExpr::AddRec {
        start: Box::new(start_scev),
        step: right.clone(),
    })
}

/// Compute SCEVs for all induction variables in a loop
pub fn compute<'a>(
    function: &'a Function,
    lop: &Loop,
    dfg: &DFGAnalysis,
) -> HashMap<Reg, SCEVExpr<'a>> {
    let mut scevs = HashMap::new();

    let block = function.block(lop.header);
    for phi in &block.phi_nodes {
        if let Some(addrec) = try_build_addrec(function, dfg, phi, lop) {
            scevs.insert(phi.dest, addrec);
        }
    }

    scevs
}
