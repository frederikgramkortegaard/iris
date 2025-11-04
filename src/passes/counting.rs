use crate::ast::{Expression, Program, Statement};
use crate::types::{Function, Variable};
use crate::visitor::{DiagnosticCollector, Visitor};

/// Example visitor that counts different types of nodes in the AST
pub struct CountingPass {
    pub num_functions: usize,
    pub num_statements: usize,
    pub num_expressions: usize,
    pub num_variables: usize,
    diagnostics: DiagnosticCollector,
}

impl CountingPass {
    pub fn new() -> Self {
        CountingPass {
            num_functions: 0,
            num_statements: 0,
            num_expressions: 0,
            num_variables: 0,
            diagnostics: DiagnosticCollector::new(),
        }
    }
}

impl Visitor for CountingPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_program(&mut self, program: &mut Program) -> () {
        // Walk the AST to count everything
        self.walk_program(program);

        // Report the counts
        self.diagnostics
            .info(format!("Functions: {}", self.num_functions));
        self.diagnostics
            .info(format!("Statements: {}", self.num_statements));
        self.diagnostics
            .info(format!("Expressions: {}", self.num_expressions));
        self.diagnostics
            .info(format!("Variables: {}", self.num_variables));
    }

    fn visit_function(&mut self, function: &mut Function) -> () {
        self.num_functions += 1;
        self.walk_function(function);
    }

    fn visit_statement(&mut self, statement: &mut Statement) -> () {
        self.num_statements += 1;
        self.walk_statement(statement);
    }

    fn visit_expression(&mut self, expression: &mut Expression) -> () {
        self.num_expressions += 1;
        self.walk_expression(expression);
    }

    fn visit_variable(&mut self, variable: &mut Variable) -> () {
        self.num_variables += 1;
        self.walk_variable(variable);
    }
}
