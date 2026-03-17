use crate::codegen::wasm::types::{WatInstruction, WatModule};

/// Recurse into nested control flow bodies.
fn optimize_nested(inst: WatInstruction) -> WatInstruction {
    match inst {
        WatInstruction::Block { label, body } => WatInstruction::Block {
            label,
            body: optimize(body),
        },
        WatInstruction::Loop { label, body } => WatInstruction::Loop {
            label,
            body: optimize(body),
        },
        WatInstruction::If {
            then_body,
            else_body,
        } => WatInstruction::If {
            then_body: optimize(then_body),
            else_body: optimize(else_body),
        },
        other => other,
    }
}

/// Peephole optimize a flat instruction list.
/// - LocalSet(x) followed by LocalGet(x): remove both, leave value on stack.
fn optimize(instructions: Vec<WatInstruction>) -> Vec<WatInstruction> {
    let mut result = Vec::with_capacity(instructions.len());
    let mut iter = instructions.into_iter().peekable();

    while let Some(inst) = iter.next() {
        match (&inst, iter.peek()) {
            (WatInstruction::LocalSet(a), Some(WatInstruction::LocalGet(b))) if a == b => {
                iter.next(); // consume the LocalGet
            }
            _ => {
                result.push(optimize_nested(inst));
            }
        }
    }

    result
}

pub fn peephole(module: &mut WatModule) {
    for func in &mut module.functions {
        func.body = optimize(std::mem::take(&mut func.body));
    }
}
