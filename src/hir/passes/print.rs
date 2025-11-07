use crate::ast::{Expression, Program, Statement};
use crate::hir::visitor::{DiagnosticCollector, Visitor};
use crate::span::Span;
use crate::types::{Function, Variable};

/// Visitor that prints the AST structure with indentation
pub struct PrintPass {
    indent: usize,
    diagnostics: DiagnosticCollector,
}

impl PrintPass {
    pub fn new() -> Self {
        PrintPass {
            indent: 0,
            diagnostics: DiagnosticCollector::new(),
        }
    }


    fn print(&self, msg: &str) {
        println!("{}{}", "  ".repeat(self.indent), msg);
    }

    fn format_span(span: &Span) -> String {
        if span.start_row == span.end_row {
            format!(
                "{}:{}-{}",
                span.start_row, span.start_column, span.end_column
            )
        } else {
            format!(
                "{}:{}-{}:{}",
                span.start_row, span.start_column, span.end_row, span.end_column
            )
        }
    }

    fn indent(&mut self) {
        self.indent += 1;
    }

    fn dedent(&mut self) {
        if self.indent > 0 {
            self.indent -= 1;
        }
    }
}

impl Visitor for PrintPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_program(&mut self, program: &mut Program) -> () {
        self.print(&format!(
            "Program ({} globals, {} functions)",
            program.globals.len(),
            program.functions.len()
        ));
        self.indent();
        self.walk_program(program);
        self.dedent();
    }

    fn visit_function(&mut self, function: &mut Function) -> () {
        self.print(&format!("Function: {}", function.name));
        self.indent();
        self.walk_function(function);
        self.dedent();
    }

    fn visit_variable(&mut self, variable: &mut Variable) -> () {
        self.print(&format!("Variable: {}", variable.name));
        self.indent();
        self.walk_variable(variable);
        self.dedent();
    }

    fn visit_statement(&mut self, statement: &mut Statement) -> () {
        match statement {
            Statement::Assignment { left, span, .. } => self.print(&format!(
                "Assignment to: {} @ {}",
                left,
                Self::format_span(span)
            )),
            Statement::FunctionDefinition { name, span, .. } => self.print(&format!(
                "FunctionDef: {} @ {}",
                name,
                Self::format_span(span)
            )),
            Statement::If { span, .. } => {
                self.print(&format!("If statement @ {}", Self::format_span(span)))
            }
            Statement::While { span, .. } => {
                self.print(&format!("While loop @ {}", Self::format_span(span)))
            }
            Statement::Block { span, .. } => {
                self.print(&format!("Block @ {}", Self::format_span(span)))
            }
            Statement::Return { span, .. } => {
                self.print(&format!("Return @ {}", Self::format_span(span)))
            }
            Statement::Expression { span, .. } => self.print(&format!(
                "Expression statement @ {}",
                Self::format_span(span)
            )),
        }
        self.indent();
        self.walk_statement(statement);
        self.dedent();
    }

    fn visit_expression(&mut self, expression: &mut Expression) -> () {
        match expression {
            Expression::Number { value: n, span, .. } => {
                self.print(&format!("Number: {} @ {}", n, Self::format_span(span)))
            }
            Expression::Boolean { value: b, span, .. } => {
                self.print(&format!("Boolean: {} @ {}", b, Self::format_span(span)))
            }
            Expression::BinaryOp { span, .. } => {
                self.print(&format!("BinaryOp @ {}", Self::format_span(span)))
            }
            Expression::UnaryOp { span, .. } => {
                self.print(&format!("UnaryOp @ {}", Self::format_span(span)))
            }
            Expression::Call {
                identifier,
                args,
                span,
                ..
            } => self.print(&format!(
                "Call: {}({} args) @ {}",
                identifier,
                args.len(),
                Self::format_span(span)
            )),
            Expression::Variable {
                name: identifier,
                span,
                ..
            } => self.print(&format!(
                "Variable ref: {} @ {}",
                identifier,
                Self::format_span(span)
            )),
        }
        self.indent();
        self.walk_expression(expression);
        self.dedent();
    }
}
