use crate::ast::{Expression, Program, Statement};
use crate::types::{Function, Variable};
use crate::visitor::{DiagnosticCollector, Visitor};

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
            Statement::Assignment { left, .. } => {
                self.print(&format!("Assignment to: {}", left))
            }
            Statement::FunctionDefinition { name, .. } => {
                self.print(&format!("FunctionDef: {}", name))
            }
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

    fn visit_expression(&mut self, expression: &mut Expression) -> () {
        match expression {
            Expression::Number(n) => self.print(&format!("Number: {}", n)),
            Expression::BinaryOp { .. } => self.print("BinaryOp"),
            Expression::UnaryOp { .. } => self.print("UnaryOp"),
            Expression::Call { identifier, args } => {
                self.print(&format!("Call: {}({} args)", identifier, args.len()))
            }
            Expression::Variable(identifier) => {
                self.print(&format!("Variable ref: {}", identifier))
            }
        }
        self.indent();
        self.walk_expression(expression);
        self.dedent();
    }
}
