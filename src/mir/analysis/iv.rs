use crate::mir::analysis::dfg::DFGAnalysis;
use crate::mir::analysis::loops::{InductionVar, Loop};
use crate::mir::{Function, Instruction, Opcode, Operand, Reg};
use std::collections::HashMap;

/// Internal SCEV expression used during analysis
#[derive(Debug, Clone)]
enum SCEVExpr<'a> {
    BinOp {
        kind: Opcode,
        left: Box<SCEVExpr<'a>>,
        right: Box<SCEVExpr<'a>>,
    },
    Unknown,
    Constant(&'a Operand),
    PhiSelf,
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
                return SCEVExpr::PhiSelf;
            }

            let Some(inst) = dfg.get_instruction(f, *r) else {
                return SCEVExpr::Unknown;
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
                _ => SCEVExpr::Unknown,
            }
        }
        _ => SCEVExpr::Unknown,
    }
}

fn try_build_iv(
    f: &Function,
    dfg: &DFGAnalysis,
    phi: &Instruction,
    lop: &Loop,
) -> Option<InductionVar> {
    // Only two-arg phi nodes (simple case)
    if phi.args.len() != 2 {
        return None;
    }

    let (mut start_op, mut back_op) = (None, None);
    for arg in &phi.args {
        let Operand::Pair(blockid, op) = arg else {
            continue;
        };
        if lop.body.contains(blockid) {
            back_op = Some(op.as_ref());
        } else {
            start_op = Some(op.as_ref());
        }
    }

    let start_op = start_op?;
    let back_op = back_op?;

    // Analyze back edge expression
    let back_scev = build(f, back_op, dfg, phi.dest);

    let SCEVExpr::BinOp { kind, left, right } = &back_scev else {
        return None;
    };

    if *kind != Opcode::Add {
        return None;
    }

    // Left must be self-reference
    let SCEVExpr::PhiSelf = left.as_ref() else {
        return None;
    };

    // Right (step) must be constant
    let SCEVExpr::Constant(step) = right.as_ref() else {
        return None;
    };

    // Start must be constant
    let start_scev = build(f, start_op, dfg, phi.dest);
    let SCEVExpr::Constant(start) = &start_scev else {
        return None;
    };

    Some(InductionVar {
        start: (*start).clone(),
        step: (*step).clone(),
    })
}

/// Compute induction variables for a loop
pub fn compute(
    function: &Function,
    lop: &Loop,
    dfg: &DFGAnalysis,
) -> HashMap<Reg, InductionVar> {
    let mut ivs = HashMap::new();

    let block = function.block(lop.header);
    for phi in &block.phi_nodes {
        if let Some(iv) = try_build_iv(function, dfg, phi, lop) {
            ivs.insert(phi.dest, iv);
        }
    }

    ivs
}
