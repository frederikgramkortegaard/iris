use crate::codegen::wasm::types::{WatFunction, WatInstruction, WatModule, WatType};
use crate::mir::analysis::cfg::{
    compute_cfg, compute_dominator_tree, compute_dominators, compute_rpo, DominatorSets,
    DominatorTree, Predecessors, Successors,
};
use crate::mir::{BlockId, Function, Instruction, Opcode, Operand, Program, Reg, Terminator, Type};
use std::collections::{BTreeMap, HashMap, HashSet};

fn mir_type_to_wat(typ: Type) -> WatType {
    match typ {
        Type::F64 => WatType::F64,
        Type::F32 | Type::F16 | Type::F8 => WatType::F32,
        Type::I64 => WatType::I64,
        Type::I1 | Type::I8 | Type::I16 | Type::I32 => WatType::I32,
        Type::Void => WatType::I32,
    }
}

/// Push an operand onto the wasm stack.
/// Reg becomes local.get, immediates become const instructions.
fn lower_operand(operand: &Operand) -> WatInstruction {
    match operand {
        Operand::Reg(r) => WatInstruction::LocalGet(*r as u32),
        Operand::ImmF64(v) => WatInstruction::F64Const(*v),
        Operand::ImmI64(v) => WatInstruction::I32Const(*v as i32),
        Operand::ImmBool(b) => WatInstruction::I32Const(if *b { 1 } else { 0 }),
        Operand::Label(_) | Operand::Pair(_, _) => {
            unreachable!("Labels and Pairs should not appear in lowered MIR")
        }
    }
}

/// Pick the right arithmetic/comparison WAT instruction based on the MIR type.
fn typed_binop(op: &Opcode, typ: Type) -> WatInstruction {
    let wat_type = mir_type_to_wat(typ);
    match (op, wat_type) {
        (Opcode::Add, WatType::F64) => WatInstruction::F64Add,
        (Opcode::Sub, WatType::F64) => WatInstruction::F64Sub,
        (Opcode::Mul, WatType::F64) => WatInstruction::F64Mul,
        (Opcode::Div, WatType::F64) => WatInstruction::F64Div,

        (Opcode::Add, WatType::F32) => WatInstruction::F32Add,
        (Opcode::Sub, WatType::F32) => WatInstruction::F32Sub,
        (Opcode::Mul, WatType::F32) => WatInstruction::F32Mul,
        (Opcode::Div, WatType::F32) => WatInstruction::F32Div,

        (Opcode::Eq, WatType::F64) => WatInstruction::F64Eq,
        (Opcode::Ne, WatType::F64) => WatInstruction::F64Ne,
        (Opcode::Lt, WatType::F64) => WatInstruction::F64Lt,
        (Opcode::Le, WatType::F64) => WatInstruction::F64Le,
        (Opcode::Gt, WatType::F64) => WatInstruction::F64Gt,
        (Opcode::Ge, WatType::F64) => WatInstruction::F64Ge,

        (Opcode::Eq, WatType::F32) => WatInstruction::F32Eq,
        (Opcode::Ne, WatType::F32) => WatInstruction::F32Ne,
        (Opcode::Lt, WatType::F32) => WatInstruction::F32Lt,
        (Opcode::Le, WatType::F32) => WatInstruction::F32Le,
        (Opcode::Gt, WatType::F32) => WatInstruction::F32Gt,
        (Opcode::Ge, WatType::F32) => WatInstruction::F32Ge,

        (Opcode::Add, WatType::I32) => WatInstruction::I32Add,
        (Opcode::Sub, WatType::I32) => WatInstruction::I32Sub,
        (Opcode::Mul, WatType::I32) => WatInstruction::I32Mul,
        (Opcode::Div, WatType::I32) => WatInstruction::I32DivS,

        (Opcode::Eq, WatType::I32) => WatInstruction::I32Eq,
        (Opcode::Ne, WatType::I32) => WatInstruction::I32Ne,
        (Opcode::Lt, WatType::I32) => WatInstruction::I32LtS,
        (Opcode::Le, WatType::I32) => WatInstruction::I32LeS,
        (Opcode::Gt, WatType::I32) => WatInstruction::I32GtS,
        (Opcode::Ge, WatType::I32) => WatInstruction::I32GeS,

        _ => unreachable!("Unsupported op {:?} for type {:?}", op, wat_type),
    }
}

/// Lower a single MIR instruction to WAT: push operands, apply op, local.set dest.
fn lower_instruction(inst: &Instruction, function: &Function) -> Vec<WatInstruction> {
    let dest = inst.dest as u32;

    match &inst.op {
        // Copy: push source, store to dest
        Opcode::Copy => {
            vec![lower_operand(&inst.args[0]), WatInstruction::LocalSet(dest)]
        }

        // Arithmetic: dispatch on result type (inst.typ)
        Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div => {
            vec![
                lower_operand(&inst.args[0]),
                lower_operand(&inst.args[1]),
                typed_binop(&inst.op, inst.typ),
                WatInstruction::LocalSet(dest),
            ]
        }

        // Comparisons: result is always I1, but the WAT instruction depends
        // on the operand type (f64.gt vs i32.gt_s), not the result type.
        // Look up the operand type from the first argument.
        Opcode::Eq | Opcode::Ne | Opcode::Lt | Opcode::Le | Opcode::Gt | Opcode::Ge => {
            let operand_type = match &inst.args[0] {
                Operand::Reg(r) => function.reg_type(*r).expect("Unknown register type"),
                Operand::ImmF64(_) => Type::F64,
                Operand::ImmI64(_) => Type::I64,
                Operand::ImmBool(_) => Type::I1,
                _ => unreachable!("Unexpected operand in comparison"),
            };
            vec![
                lower_operand(&inst.args[0]),
                lower_operand(&inst.args[1]),
                typed_binop(&inst.op, operand_type),
                WatInstruction::LocalSet(dest),
            ]
        }

        // Call: push each argument, call, store result
        Opcode::Call => {
            let mut wat = Vec::new();
            // First arg is the label (function name), rest are actual arguments
            let func_name = match &inst.args[0] {
                Operand::Label(name) => name.clone(),
                _ => unreachable!("Call first arg must be a Label"),
            };
            for arg in &inst.args[1..] {
                wat.push(lower_operand(arg));
            }
            wat.push(WatInstruction::Call(func_name));
            if inst.typ != Type::Void {
                wat.push(WatInstruction::LocalSet(dest));
            }
            wat
        }

        Opcode::Mod => todo!("Mod not yet supported in WAT lowering"),
        Opcode::Phi => unreachable!("Phi nodes should be eliminated before WAT lowering"),
    }
}

/// Lower a basic block's instructions and terminator.
/// Br/BrIf terminators are skipped; the structuring walk handles control flow.
fn lower_block(block_id: BlockId, function: &Function) -> Vec<WatInstruction> {
    let block = function.arena.get(block_id);
    let mut wat = Vec::new();

    // Lower all instructions in the block
    for inst in &block.instructions {
        wat.extend(lower_instruction(inst, function));
    }

    // Handle terminator
    match &block.terminator {
        Terminator::Ret { value } => {
            if let Some(val) = value {
                wat.push(lower_operand(val));
            }
            wat.push(WatInstruction::Return);
        }
        Terminator::Unreachable => {
            wat.push(WatInstruction::Unreachable);
        }
        // Br/BrIf are handled by the StructuredNode walk
        Terminator::Br { .. } | Terminator::BrIf { .. } => {}
    }

    wat
}

#[derive(Debug, Clone)]
pub enum ContainingStack {
    BlockFollowedBy { followed_by: BlockId },
    LoopHeadedBy { headed_by: BlockId },
    IfThenElse,
}

/*
*do_tree(X, ctx):
    kids = dominator children of X that are merge nodes, RPO ascending
    if X is a loop header:
        loop { node_within(X, kids, LoopHeadedBy(X) :: ctx) }
    else:
        node_within(X, kids, ctx)
*
*
*
* node_within(X, kids, ctx):
    if kids not empty:
        Y = kids.last()                       // largest RPO: outermost
        block { node_within(X, kids - Y, BlockFollowedBy(Y) :: ctx) }
        do_tree(Y, ctx)                       // Y starts right after end
    else:
        X's instructions
        then X's terminator:
            Br T        -> do_branch(X, T, ctx)
            BrIf c,A,B  -> push c
                           if   { do_branch(X, A, IfThenElse :: ctx) }
                           else { do_branch(X, B, IfThenElse :: ctx) }
            Ret         -> return
            Unreachable -> unreachable
* */

/// Branch depth for a br naming `target`: the number of constructs between
/// here and the one whose landing point is `target`, innermost first.
/// Panics if no enclosing construct targets it -- that means the nesting
/// built by do_tree/node_within is wrong (or the CFG is irreducible).
fn index(target: BlockId, context: &[ContainingStack]) -> u32 {
    context
        .iter()
        .rev()
        .position(|frame| match frame {
            ContainingStack::BlockFollowedBy { followed_by } => *followed_by == target,
            ContainingStack::LoopHeadedBy { headed_by } => *headed_by == target,
            ContainingStack::IfThenElse => false,
        })
        .unwrap_or_else(|| panic!("no enclosing construct targets {:?}", target)) as u32
}

#[allow(clippy::too_many_arguments)]
fn do_branch(
    function: &Function,
    preds: &Predecessors,
    doms: &DominatorSets,
    rpo_of: &HashMap<BlockId, usize>,
    dom_kids: &BTreeMap<BlockId, Vec<BlockId>>,
    block_id: BlockId,
    target: BlockId,
    context: &mut Vec<ContainingStack>,
) -> Vec<WatInstruction> {
    if rpo_of[&target] <= rpo_of[&block_id] {
        // backedge (or self-edge): the loop frame is in context
        vec![WatInstruction::Br(index(target, context))]
    } else if preds[&target].len() > 1 {
        // forward edge to a merge node: its wrapper block is in context
        vec![WatInstruction::Br(index(target, context))]
    } else {
        // we are the only way in: place target's code right here
        do_tree(function, preds, doms, rpo_of, dom_kids, target, context)
    }
}

#[allow(clippy::too_many_arguments)]
fn node_within(
    function: &Function,
    preds: &Predecessors,
    doms: &DominatorSets,
    rpo_of: &HashMap<BlockId, usize>,
    dom_kids: &BTreeMap<BlockId, Vec<BlockId>>,
    block_id: BlockId,
    mut kids: Vec<BlockId>,
    context: &mut Vec<ContainingStack>,
) -> Vec<WatInstruction> {
    let mut out = vec![];
    if let Some(y) = kids.pop() {
        // largest RPO: outermost wrapper
        context.push(ContainingStack::BlockFollowedBy { followed_by: y });
        out.push(WatInstruction::Block {
            label: None,
            body: node_within(
                function, preds, doms, rpo_of, dom_kids, block_id, kids, context,
            ),
        });
        context.pop();

        out.extend(do_tree(function, preds, doms, rpo_of, dom_kids, y, context));
    } else {
        out.extend(lower_block(block_id, function)); // X's actual instructions
        match &function.arena.get(block_id).terminator {
            Terminator::Br { target } => {
                out.extend(do_branch(
                    function, preds, doms, rpo_of, dom_kids, block_id, *target, context,
                ));
            }
            Terminator::BrIf {
                cond,
                then_bb,
                else_bb,
            } => {
                out.push(lower_operand(cond));
                context.push(ContainingStack::IfThenElse);
                let then_body = do_branch(
                    function, preds, doms, rpo_of, dom_kids, block_id, *then_bb, context,
                );
                let else_body = do_branch(
                    function, preds, doms, rpo_of, dom_kids, block_id, *else_bb, context,
                );
                context.pop();
                out.push(WatInstruction::If {
                    then_body,
                    else_body,
                });
            }
            // lower_block already emitted these
            Terminator::Ret { .. } | Terminator::Unreachable => {}
        }
    }

    out
}

#[allow(clippy::too_many_arguments)]
fn do_tree(
    function: &Function,
    preds: &Predecessors,
    doms: &DominatorSets,
    rpo_of: &HashMap<BlockId, usize>,
    dom_kids: &BTreeMap<BlockId, Vec<BlockId>>,
    block_id: BlockId,
    context: &mut Vec<ContainingStack>,
) -> Vec<WatInstruction> {
    // dominator children of X that are merge nodes, RPO ascending
    let kids: Vec<BlockId> = dom_kids
        .get(&block_id)
        .into_iter()
        .flatten()
        .filter(|k| preds[k].len() > 1)
        .copied()
        .collect();

    // block_id is a loop header
    if preds[&block_id].iter().any(|u| doms[u].contains(&block_id)) {
        context.push(ContainingStack::LoopHeadedBy {
            headed_by: block_id,
        });
        let body = node_within(
            function, preds, doms, rpo_of, dom_kids, block_id, kids, context,
        );
        context.pop();
        vec![WatInstruction::Loop { label: None, body }]
    } else {
        node_within(
            function, preds, doms, rpo_of, dom_kids, block_id, kids, context,
        )
    }
}

pub fn ramsey_structured_codegen(
    function: &Function,
    entry: BlockId,
    doms: &DominatorSets,
    dom_tree: &DominatorTree,
    succs: &Successors,
    preds: &Predecessors,
) -> Vec<WatInstruction> {
    let rpo = compute_rpo(entry, succs);
    let rpo_of: HashMap<BlockId, usize> =
        rpo.iter().enumerate().map(|(e, bid)| (*bid, e)).collect();

    // dom_tree maps child -> idom, the recursion wants parent -> children
    let mut dom_kids: BTreeMap<BlockId, Vec<BlockId>> = BTreeMap::new();
    for (&child, &parent) in dom_tree {
        dom_kids.entry(parent).or_default().push(child);
    }
    for v in dom_kids.values_mut() {
        v.sort_by_key(|b| rpo_of[b]);
    }

    do_tree(
        function,
        preds,
        doms,
        &rpo_of,
        &dom_kids,
        entry,
        &mut vec![],
    )
}

fn lower_function(function: &Function) -> WatFunction {
    let params = function
        .params
        .iter()
        .map(|(reg, typ)| (format!("r{}", reg), mir_type_to_wat(*typ)))
        .collect();

    let result = if function.return_type != Type::Void {
        Some(mir_type_to_wat(function.return_type))
    } else {
        None
    };

    // Collect locals: all dest registers that aren't params
    let param_regs: HashSet<Reg> = function.params.iter().map(|(r, _)| *r).collect();
    let mut local_regs: BTreeMap<Reg, Type> = BTreeMap::new();
    for (_, block) in function.arena.iter() {
        for inst in &block.instructions {
            if !param_regs.contains(&inst.dest) {
                local_regs.insert(inst.dest, inst.typ);
            }
        }
    }
    let locals = local_regs
        .iter()
        .map(|(reg, typ)| (format!("r{}", reg), mir_type_to_wat(*typ)))
        .collect();

    // Compute CFG and dominator tree, then run Ramsey structuring.
    // Start at the virtual entry, not the real entry. After phi elimination
    // the virtual entry holds the copies that set up loop variables from
    // the parameters, dropping it meant those never ran.
    let (preds, succs) = compute_cfg(function);
    let dom_sets = compute_dominators(function, &preds);
    let dom_tree = compute_dominator_tree(function, &dom_sets, &succs);

    let mut body = ramsey_structured_codegen(
        function,
        function.virtual_entry,
        &dom_sets,
        &dom_tree,
        &succs,
        &preds,
    );

    // if every path returns this point is dead, but the validator still
    // wants the stack to match the result type at the end
    if result.is_some() {
        body.push(WatInstruction::Unreachable);
    }

    WatFunction {
        name: function.name.clone(),
        params,
        result,
        locals,
        body,
    }
}

pub fn lower(program: &Program) -> WatModule {
    let functions = program
        .functions
        .iter()
        .map(|(_, f)| lower_function(f))
        .collect();
    WatModule { functions }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_counts_frames_from_innermost() {
        // stack grows left to right: Loop(9) is outermost, the Ifs innermost
        let ctx = vec![
            ContainingStack::LoopHeadedBy {
                headed_by: BlockId::new(9),
            },
            ContainingStack::BlockFollowedBy {
                followed_by: BlockId::new(15),
            },
            ContainingStack::IfThenElse,
            ContainingStack::IfThenElse,
        ];
        assert_eq!(index(BlockId::new(15), &ctx), 2);
        assert_eq!(index(BlockId::new(9), &ctx), 3);
    }

    #[test]
    #[should_panic]
    fn index_panics_on_missing_target() {
        let ctx = vec![ContainingStack::IfThenElse];
        index(BlockId::new(42), &ctx);
    }
}
