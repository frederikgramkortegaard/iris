// Example showing how to use the Visitor pattern
// Run with: cargo run --example use_visitor

use iris::lexer::LexerContext;
use iris::parser::ParserContext;
use iris::visitor::{Visitor, DiagnosticCollector};
use iris::ast::{Expression, Statement, Program};
use iris::types::{Function, Variable};

/// Simple visitor that prints what it visits
struct PrintVisitor {
    indent: usize,
    diagnostics: DiagnosticCollector,
}

impl PrintVisitor {
    fn new() -> Self {
        PrintVisitor {
            indent: 0,
            diagnostics: DiagnosticCollector::new(),
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
}

impl Visitor for PrintVisitor {
    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_program(&mut self, program: &Program) {
        self.print(&format!("Program ({} globals, {} functions)",
            program.globals.len(),
            program.functions.len()));
        self.indent();
        self.walk_program(program);
        self.dedent();
    }

    fn visit_function(&mut self, function: &Function) {
        self.print(&format!("Function: {}", function.name));
        self.indent();
        self.walk_function(function);
        self.dedent();
    }

    fn visit_variable(&mut self, variable: &Variable) {
        self.print(&format!("Variable: {}", variable.name));
        self.indent();
        self.walk_variable(variable);
        self.dedent();
    }

    fn visit_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Assignment { left, .. } => self.print(&format!("Assignment to: {}", left)),
            Statement::FunctionDefinition { name, .. } => self.print(&format!("FunctionDef: {}", name)),
            Statement::If { .. } => self.print("If statement"),
            Statement::While { .. } => self.print("While loop"),
            Statement::Block(_) => self.print("Block"),
            Statement::Return(_) => self.print("Return"),
            Statement::Expression(_) => self.print("Expression statement"),
        }
        self.indent();
        self.walk_statement(statement);
        self.dedent();
    }

    fn visit_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::Number(n) => self.print(&format!("Number: {}", n)),
            Expression::BinaryOp { .. } => self.print("BinaryOp"),
            Expression::UnaryOp { .. } => self.print("UnaryOp"),
            Expression::Call { identifier, args } => {
                self.print(&format!("Call: {}({} args)", identifier, args.len()))
            }
            Expression::Variable { identifier } => self.print(&format!("Variable ref: {}", identifier)),
        }
        self.indent();
        self.walk_expression(expression);
        self.dedent();
    }
}

fn main() {
    let source = r#"
fn factorial(n: f64) -> f64 {
  if (n <= 1) {
    return 1
  } else {
    return n * factorial(n - 1)
  }
}
"#;

    // Lex and parse
    let tokens = LexerContext::lex(source).expect("Lexing failed");
    let mut parser = ParserContext::new(tokens);
    let program = parser.parse().expect("Parsing failed");

    // Use the visitor
    let mut visitor = PrintVisitor::new();
    visitor.visit_program(&program);
}
