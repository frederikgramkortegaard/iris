use crate::codegen::wasm::types::{WatFunction, WatInstruction, WatModule, WatType};
use crate::codegen::Emit;

fn emit_type(typ: &WatType) -> &'static str {
    match typ {
        WatType::I32 => "i32",
        WatType::I64 => "i64",
        WatType::F32 => "f32",
        WatType::F64 => "f64",
    }
}

fn emit_instruction(inst: &WatInstruction, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    match inst {
        // Constants
        WatInstruction::I32Const(v) => format!("{pad}i32.const {v}"),
        WatInstruction::F32Const(v) => format!("{pad}f32.const {v}"),
        WatInstruction::F64Const(v) => format!("{pad}f64.const {v}"),

        // Variables
        WatInstruction::LocalGet(i) => format!("{pad}local.get $r{i}"),
        WatInstruction::LocalSet(i) => format!("{pad}local.set $r{i}"),

        // f64 arithmetic
        WatInstruction::F64Add => format!("{pad}f64.add"),
        WatInstruction::F64Sub => format!("{pad}f64.sub"),
        WatInstruction::F64Mul => format!("{pad}f64.mul"),
        WatInstruction::F64Div => format!("{pad}f64.div"),

        // f32 arithmetic
        WatInstruction::F32Add => format!("{pad}f32.add"),
        WatInstruction::F32Sub => format!("{pad}f32.sub"),
        WatInstruction::F32Mul => format!("{pad}f32.mul"),
        WatInstruction::F32Div => format!("{pad}f32.div"),

        // i32 arithmetic
        WatInstruction::I32Add => format!("{pad}i32.add"),
        WatInstruction::I32Sub => format!("{pad}i32.sub"),
        WatInstruction::I32Mul => format!("{pad}i32.mul"),
        WatInstruction::I32DivS => format!("{pad}i32.div_s"),

        // f64 comparisons
        WatInstruction::F64Eq => format!("{pad}f64.eq"),
        WatInstruction::F64Ne => format!("{pad}f64.ne"),
        WatInstruction::F64Lt => format!("{pad}f64.lt"),
        WatInstruction::F64Le => format!("{pad}f64.le"),
        WatInstruction::F64Gt => format!("{pad}f64.gt"),
        WatInstruction::F64Ge => format!("{pad}f64.ge"),

        // f32 comparisons
        WatInstruction::F32Eq => format!("{pad}f32.eq"),
        WatInstruction::F32Ne => format!("{pad}f32.ne"),
        WatInstruction::F32Lt => format!("{pad}f32.lt"),
        WatInstruction::F32Le => format!("{pad}f32.le"),
        WatInstruction::F32Gt => format!("{pad}f32.gt"),
        WatInstruction::F32Ge => format!("{pad}f32.ge"),

        // i32 comparisons
        WatInstruction::I32Eq => format!("{pad}i32.eq"),
        WatInstruction::I32Ne => format!("{pad}i32.ne"),
        WatInstruction::I32LtS => format!("{pad}i32.lt_s"),
        WatInstruction::I32LeS => format!("{pad}i32.le_s"),
        WatInstruction::I32GtS => format!("{pad}i32.gt_s"),
        WatInstruction::I32GeS => format!("{pad}i32.ge_s"),
        WatInstruction::I32Eqz => format!("{pad}i32.eqz"),

        // Control flow
        WatInstruction::Br(depth) => format!("{pad}br {depth}"),
        WatInstruction::BrIf(depth) => format!("{pad}br_if {depth}"),
        WatInstruction::Return => format!("{pad}return"),
        WatInstruction::Unreachable => format!("{pad}unreachable"),
        WatInstruction::Call(name) => format!("{pad}call ${name}"),

        WatInstruction::Block { label, body } => {
            let label_str = label.as_ref().map(|l| format!(" ${l}")).unwrap_or_default();
            let mut lines = vec![format!("{pad}block{label_str}")];
            for inst in body {
                lines.push(emit_instruction(inst, indent + 1));
            }
            lines.push(format!("{pad}end"));
            lines.join("\n")
        }

        WatInstruction::Loop { label, body } => {
            let label_str = label.as_ref().map(|l| format!(" ${l}")).unwrap_or_default();
            let mut lines = vec![format!("{pad}loop{label_str}")];
            for inst in body {
                lines.push(emit_instruction(inst, indent + 1));
            }
            lines.push(format!("{pad}end"));
            lines.join("\n")
        }

        WatInstruction::If {
            then_body,
            else_body,
        } => {
            let mut lines = vec![format!("{pad}if")];
            for inst in then_body {
                lines.push(emit_instruction(inst, indent + 1));
            }
            if !else_body.is_empty() {
                lines.push(format!("{pad}else"));
                for inst in else_body {
                    lines.push(emit_instruction(inst, indent + 1));
                }
            }
            lines.push(format!("{pad}end"));
            lines.join("\n")
        }
    }
}

fn emit_function(func: &WatFunction) -> String {
    let mut lines = Vec::new();

    // Function header with export
    lines.push(format!("  (func ${} (export \"{}\")", func.name, func.name));

    // Parameters
    for (name, typ) in &func.params {
        lines.push(format!("    (param ${} {})", name, emit_type(typ)));
    }

    // Result
    if let Some(typ) = &func.result {
        lines.push(format!("    (result {})", emit_type(typ)));
    }

    // Locals
    for (name, typ) in &func.locals {
        lines.push(format!("    (local ${} {})", name, emit_type(typ)));
    }

    // Body
    for inst in &func.body {
        lines.push(emit_instruction(inst, 2));
    }

    lines.push("  )".to_string());
    lines.join("\n")
}

impl Emit for WatModule {
    fn emit(&self) -> String {
        let mut lines = vec!["(module".to_string()];
        for func in &self.functions {
            lines.push(emit_function(func));
        }
        lines.push(")".to_string());
        lines.join("\n")
    }
}
