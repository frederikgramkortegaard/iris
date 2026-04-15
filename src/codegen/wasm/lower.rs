use crate::codegen::wasm::ramsey::ramsey_structuring;
use crate::codegen::wasm::types::{WatFunction, WatInstruction, WatModule, WatType};
use crate::codegen::wasm::StructuredNode;
use crate::mir::analysis::cfg::{compute_cfg, compute_dominator_tree, compute_dominators};
use crate::mir::{Function, Instruction, Opcode, Operand, Program, Reg, Terminator, Type};
use std::collections::{BTreeMap, HashSet};

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
/// Br/BrIf terminators are skipped; the parent StructuredNode handles control flow.
fn lower_block(block_id: crate::mir::BlockId, function: &Function) -> Vec<WatInstruction> {
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

/// Walk a StructuredNode tree, emitting WAT instructions.
fn lower_node(node: &StructuredNode, function: &Function) -> Vec<WatInstruction> {
    match node {
        StructuredNode::Block(block_id) => lower_block(*block_id, function),

        StructuredNode::Sequence(nodes) => {
            let mut wat = Vec::new();
            for child in nodes {
                wat.extend(lower_node(child, function));
            }
            wat
        }

        StructuredNode::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let block = function.arena.get(*cond);
            let mut wat = Vec::new();

            // Lower the condition block's instructions
            for inst in &block.instructions {
                wat.extend(lower_instruction(inst, function));
            }

            // Push the condition value from the BrIf terminator
            match &block.terminator {
                Terminator::BrIf { cond, .. } => {
                    wat.push(lower_operand(cond));
                }
                _ => unreachable!("If node's cond block must end with BrIf"),
            }

            // Emit if/else with recursively lowered branches
            wat.push(WatInstruction::If {
                then_body: lower_node(then_branch, function),
                else_body: lower_node(else_branch, function),
            });
            // Both branches return, so this point is never reached.
            wat.push(WatInstruction::Unreachable);
            wat
        }

        StructuredNode::Loop { header, body } => {
            let block = function.arena.get(*header);
            let mut header_wat = Vec::new();

            // Lower header block's instructions
            for inst in &block.instructions {
                header_wat.extend(lower_instruction(inst, function));
            }

            // Header ends with BrIf { cond, then_bb: body, else_bb: exit }
            // cond=true means continue looping, so we invert: exit when NOT true
            match &block.terminator {
                Terminator::BrIf { cond, .. } => {
                    header_wat.push(lower_operand(cond));
                    header_wat.push(WatInstruction::I32Eqz);
                    header_wat.push(WatInstruction::BrIf(1));
                }
                Terminator::Br { .. } => {
                    // Unconditional loop, no exit condition in header
                }
                _ => unreachable!("Loop header must end with Br or BrIf"),
            }

            // Lower the loop body
            let body_wat = lower_node(body, function);

            // Combine: header + body + br 0 (back to loop top)
            let mut loop_body = header_wat;
            loop_body.extend(body_wat);
            loop_body.push(WatInstruction::Br(0));

            // Wrap: block { loop { ... } }
            vec![WatInstruction::Block {
                label: None,
                body: vec![WatInstruction::Loop {
                    label: None,
                    body: loop_body,
                }],
            }]
        }
    }
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

    // Compute CFG and dominator tree, then run Ramsey structuring
    let (preds, succs) = compute_cfg(function);
    let dom_sets = compute_dominators(function, &preds);
    let dom_tree = compute_dominator_tree(function, &dom_sets, &succs);
    let structure = ramsey_structuring(function.entry, &dom_tree, &succs);

    // Lower the structured tree into WAT instructions
    let body = lower_node(&structure, function);

    WatFunction {
        name: function.name.clone(),
        params,
        result,
        locals,
        body,
    }
}

pub fn lower(program: &Program) -> WatModule {
    let functions = program.functions.iter().map(lower_function).collect();
    WatModule { functions }
}
