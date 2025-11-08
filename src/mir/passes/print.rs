use crate::diagnostics::DiagnosticCollector;
use crate::mir::visitor::MirVisitor;
use crate::mir::{BasicBlock, BlockId, Instruction, MirFunction, MirProgram, Operand, Terminator};

/// Prints the different Functions, Blocks, Instructions and Terminators in the MIR
pub struct MirPrintingPass {
    diagnostics: DiagnosticCollector,
    indent: usize,
}

impl MirPrintingPass {
    pub fn new() -> Self {
        MirPrintingPass {
            diagnostics: DiagnosticCollector::new(),
            indent: 0,
        }
    }

    fn print(&self, msg: &str) {
        println!("{}{}", "  ".repeat(self.indent), msg);
    }

    fn indent(&mut self) {
        self.indent += 1;
    }

    fn dedent(&mut self) {
        if self.indent > 0 {
            self.indent -= 1;
        }
    }

    fn fmt_operand(&self, op: &Operand) -> String {
        match op {
            Operand::Reg(r) => format!("r{}", r),
            Operand::ImmI64(i) => format!("{}", i),
            Operand::ImmF64(f) => format!("{}", f),
            Operand::ImmBool(b) => format!("{}", b),
            Operand::Label(s) => format!("@{}", s),
            Operand::Pair(block_id, operand) => {
                format!("[{}, {}]", self.fmt_block(*block_id), self.fmt_operand(operand))
            }
        }
    }

    fn fmt_block(&self, block_id: BlockId) -> String {
        format!("block{}", block_id.index())
    }
}

impl MirVisitor for MirPrintingPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_program(&mut self, program: &mut MirProgram) -> Self::Output {
        println!(
            "=== MIR Program ({} functions) ===\n",
            program.functions.len()
        );
        self.walk_program(program);
    }

    fn visit_function(&mut self, function: &mut MirFunction) -> Self::Output {
        println!(
            "fn {}({} params) -> {:?}:",
            function.name,
            function.params.len(),
            function.return_type
        );
        self.indent();
        self.walk_function(function);
        self.dedent();
        println!(); // Blank line after function
    }

    fn visit_basicblock(&mut self, block_id: BlockId, block: &mut BasicBlock) -> Self::Output {
        println!("block{}:", block_id.index());
        self.indent();
        self.walk_basicblock(block);
        self.dedent();
    }

    fn visit_instruction(&mut self, instruction: &mut Instruction) -> Self::Output {
        let args_str = instruction
            .args
            .iter()
            .map(|arg| self.fmt_operand(arg))
            .collect::<Vec<_>>()
            .join(", ");

        self.print(&format!(
            "r{} = {:?} {:?} [{}]",
            instruction.dest, instruction.op, instruction.typ, args_str
        ));
        self.walk_instruction(instruction);
    }

    fn visit_terminator(&mut self, terminator: &mut Terminator) -> Self::Output {
        match terminator {
            Terminator::Br { target } => {
                self.print(&format!("br {}", self.fmt_block(*target)));
            }
            Terminator::BrIf {
                cond,
                then_bb,
                else_bb,
            } => {
                self.print(&format!(
                    "br_if {}, {}, {}",
                    self.fmt_operand(cond),
                    self.fmt_block(*then_bb),
                    self.fmt_block(*else_bb)
                ));
            }
            Terminator::Ret { value } => match value {
                Some(v) => self.print(&format!("ret {}", self.fmt_operand(v))),
                None => self.print("ret"),
            },
            Terminator::Unreachable => {
                self.print("unreachable");
            }
        }
        self.walk_terminator(terminator);
    }

    fn visit_operand(&mut self, _operand: &mut Operand) -> Self::Output {
        // Operands are printed inline, no need for separate visit
    }
}
