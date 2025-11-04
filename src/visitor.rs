use crate::ast::{Block, Expression, Program, Statement};
use crate::types::{Function, Type, Variable};

/// Collects diagnostic messages during AST traversal
#[derive(Default, Debug)]
pub struct DiagnosticCollector {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
    pub debug: Vec<String>,
}

impl DiagnosticCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn error(&mut self, msg: String) {
        self.errors.push(msg);
    }

    pub fn warn(&mut self, msg: String) {
        self.warnings.push(msg);
    }

    pub fn info(&mut self, msg: String) {
        self.info.push(msg);
    }

    pub fn debug(&mut self, msg: String) {
        self.debug.push(msg);
    }

    pub fn clear(&mut self) {
        self.errors.clear();
        self.warnings.clear();
        self.info.clear();
        self.debug.clear();
    }
}

/// Visitor trait for traversing the AST without mutation.
pub trait Visitor {
    /// Returns the diagnostic collector for this visitor
    fn diagnostics(&self) -> &DiagnosticCollector;

    /// Returns a mutable reference to the diagnostic collector
    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector;

    // Program and top-level
    fn visit_program(&mut self, program: &Program) {
        self.walk_program(program);
    }

    fn walk_program(&mut self, program: &Program) {
        for global in &program.globals {
            self.visit_variable(global);
        }
        for function in &program.functions {
            self.visit_function(function);
        }
    }

    // Function
    fn visit_function(&mut self, function: &Function) {
        self.walk_function(function);
    }

    fn walk_function(&mut self, function: &Function) {
        for arg in &function.args {
            self.visit_variable(arg);
        }
        self.visit_type(&function.return_type);
        self.visit_block(&function.body);
    }

    // Variable
    fn visit_variable(&mut self, variable: &Variable) {
        self.walk_variable(variable);
    }

    fn walk_variable(&mut self, variable: &Variable) {
        self.visit_type(&variable.typ);
        if let Some(init) = &variable.initializer {
            self.visit_expression(init);
        }
    }

    // Type
    fn visit_type(&mut self, _typ: &Type) {
        // Default: do nothing, types are typically leaves
    }

    // Block
    fn visit_block(&mut self, block: &Block) {
        self.walk_block(block);
    }

    fn walk_block(&mut self, block: &Block) {
        for statement in block {
            self.visit_statement(statement);
        }
    }

    // Statements
    fn visit_statement(&mut self, statement: &Statement) {
        self.walk_statement(statement);
    }

    fn walk_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Assignment { left: _, typ, right } => {
                self.visit_assignment(typ, right);
            }
            Statement::FunctionDefinition { name: _, args, return_type, body } => {
                self.visit_function_definition(args, return_type, body);
            }
            Statement::If { condition, then, els } => {
                self.visit_if(condition, then, els);
            }
            Statement::While { condition, body } => {
                self.visit_while(condition, body);
            }
            Statement::Block(block) => {
                self.visit_block(block);
            }
            Statement::Return(expr) => {
                self.visit_return(expr);
            }
            Statement::Expression(expr) => {
                self.visit_expression_statement(expr);
            }
        }
    }

    fn visit_assignment(&mut self, typ: &Option<Type>, right: &Option<Box<Expression>>) {
        if let Some(t) = typ {
            self.visit_type(t);
        }
        if let Some(expr) = right {
            self.visit_expression(expr);
        }
    }

    fn visit_function_definition(&mut self, args: &[Variable], return_type: &Type, body: &Block) {
        for arg in args {
            self.visit_variable(arg);
        }
        self.visit_type(return_type);
        self.visit_block(body);
    }

    fn visit_if(&mut self, condition: &Expression, then: &Block, els: &Option<Block>) {
        self.visit_expression(condition);
        self.visit_block(then);
        if let Some(else_block) = els {
            self.visit_block(else_block);
        }
    }

    fn visit_while(&mut self, condition: &Expression, body: &Block) {
        self.visit_expression(condition);
        self.visit_block(body);
    }

    fn visit_return(&mut self, expr: &Expression) {
        self.visit_expression(expr);
    }

    fn visit_expression_statement(&mut self, expr: &Expression) {
        self.visit_expression(expr);
    }

    // Expressions
    fn visit_expression(&mut self, expression: &Expression) {
        self.walk_expression(expression);
    }

    fn walk_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::Number(n) => {
                self.visit_number(*n);
            }
            Expression::BinaryOp { left, op: _, right } => {
                self.visit_binary_op(left, right);
            }
            Expression::UnaryOp { left, op: _ } => {
                self.visit_unary_op(left);
            }
            Expression::Call { identifier: _, args } => {
                self.visit_call(args);
            }
            Expression::Variable { identifier: _ } => {
                self.visit_variable_expr();
            }
        }
    }

    fn visit_number(&mut self, _n: f64) {
        // Default: do nothing, numbers are leaves
    }

    fn visit_binary_op(&mut self, left: &Expression, right: &Expression) {
        self.visit_expression(left);
        self.visit_expression(right);
    }

    fn visit_unary_op(&mut self, operand: &Expression) {
        self.visit_expression(operand);
    }

    fn visit_call(&mut self, args: &[Expression]) {
        for arg in args {
            self.visit_expression(arg);
        }
    }

    fn visit_variable_expr(&mut self) {
        // Default: do nothing, variable references are leaves
    }
}
