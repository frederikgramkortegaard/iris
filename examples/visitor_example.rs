use iris::ast::Program;
use iris::visitor::Visitor;

/// Example visitor that counts different types of nodes in the AST
pub struct CountingVisitor {
    pub num_functions: usize,
    pub num_statements: usize,
    pub num_expressions: usize,
    pub num_variables: usize,
}

impl CountingVisitor {
    pub fn new() -> Self {
        CountingVisitor {
            num_functions: 0,
            num_statements: 0,
            num_expressions: 0,
            num_variables: 0,
        }
    }
}

impl Visitor for CountingVisitor {
    fn visit_function(&mut self, function: &iris::types::Function) {
        self.num_functions += 1;
        self.walk_function(function);
    }

    fn visit_statement(&mut self, statement: &iris::ast::Statement) {
        self.num_statements += 1;
        self.walk_statement(statement);
    }

    fn visit_expression(&mut self, expression: &iris::ast::Expression) {
        self.num_expressions += 1;
        self.walk_expression(expression);
    }

    fn visit_variable(&mut self, variable: &iris::types::Variable) {
        self.num_variables += 1;
        self.walk_variable(variable);
    }
}

pub fn count_nodes(program: &Program) -> (usize, usize, usize, usize) {
    let mut visitor = CountingVisitor::new();
    visitor.visit_program(program);
    (
        visitor.num_functions,
        visitor.num_statements,
        visitor.num_expressions,
        visitor.num_variables,
    )
}
