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
    /// The type returned by visitor methods
    type Output: Default;

    /// Returns the diagnostic collector for this visitor
    fn diagnostics(&self) -> &DiagnosticCollector;

    /// Returns a mutable reference to the diagnostic collector
    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector;

    // Program and top-level
    fn visit_program(&mut self, program: &mut Program) -> Self::Output {
        self.walk_program(program)
    }

    fn walk_program(&mut self, program: &mut Program) -> Self::Output {
        for global in &mut program.globals {
            self.visit_variable(global);
        }
        for function in &mut program.functions {
            self.visit_function(function);
        }
        Self::Output::default()
    }

    // Function
    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        self.walk_function(function)
    }

    fn walk_function(&mut self, function: &mut Function) -> Self::Output {
        for arg in &mut function.args {
            self.visit_variable(arg);
        }
        self.visit_type(&mut function.return_type);
        self.visit_block(&mut function.body);
        Self::Output::default()
    }

    // Variable
    fn visit_variable(&mut self, variable: &mut Variable) -> Self::Output {
        self.walk_variable(variable)
    }

    fn walk_variable(&mut self, variable: &mut Variable) -> Self::Output {
        self.visit_type(&mut variable.typ);
        if let Some(init) = &mut variable.initializer {
            self.visit_expression(init);
        }
        Self::Output::default()
    }

    // Type
    fn visit_type(&mut self, _typ: &mut Type) -> Self::Output {
        // Default: do nothing, types are typically leaves
        Self::Output::default()
    }

    // Block
    fn visit_block(&mut self, block: &mut Block) -> Self::Output {
        self.walk_block(block)
    }

    fn walk_block(&mut self, block: &mut Block) -> Self::Output {
        for statement in &mut block.statements {
            self.visit_statement(statement);
        }
        Self::Output::default()
    }

    // Statements
    fn visit_statement(&mut self, statement: &mut Statement) -> Self::Output {
        self.walk_statement(statement)
    }

    fn walk_statement(&mut self, statement: &mut Statement) -> Self::Output {
        match statement {
            Statement::Assignment { left: _, typ, right } => {
                self.visit_assignment(typ, right)
            }
            Statement::FunctionDefinition { name: _, args, return_type, body } => {
                self.visit_function_definition(args, return_type, body)
            }
            Statement::If { condition, then, els } => {
                self.visit_if(condition, then, els)
            }
            Statement::While { condition, body } => {
                self.visit_while(condition, body)
            }
            Statement::Block(block) => {
                self.visit_block(block)
            }
            Statement::Return(expr) => {
                self.visit_return(expr)
            }
            Statement::Expression(expr) => {
                self.visit_expression_statement(expr)
            }
        }
    }

    fn visit_assignment(&mut self, typ: &mut Option<Type>, right: &mut Option<Box<Expression>>) -> Self::Output {
        if let Some(t) = typ {
            self.visit_type(t);
        }
        if let Some(expr) = right {
            self.visit_expression(expr);
        }
        Self::Output::default()
    }

    fn visit_function_definition(&mut self, args: &mut [Variable], return_type: &mut Type, body: &mut Block) -> Self::Output {
        for arg in args.iter_mut() {
            self.visit_variable(arg);
        }
        self.visit_type(return_type);
        self.visit_block(body);
        Self::Output::default()
    }

    fn visit_if(&mut self, condition: &mut Expression, then: &mut Block, els: &mut Option<Block>) -> Self::Output {
        self.visit_expression(condition);
        self.visit_block(then);
        if let Some(else_block) = els {
            self.visit_block(else_block);
        }
        Self::Output::default()
    }

    fn visit_while(&mut self, condition: &mut Expression, body: &mut Block) -> Self::Output {
        self.visit_expression(condition);
        self.visit_block(body);
        Self::Output::default()
    }

    fn visit_return(&mut self, expr: &mut Option<Box<Expression>>) -> Self::Output {
        if let Some(e) = expr {
            self.visit_expression(e)
        } else {
            Self::Output::default()
        }
    }

    fn visit_expression_statement(&mut self, expr: &mut Expression) -> Self::Output {
        self.visit_expression(expr)
    }

    // Expressions
    fn visit_expression(&mut self, expression: &mut Expression) -> Self::Output {
        self.walk_expression(expression)
    }

    fn walk_expression(&mut self, expression: &mut Expression) -> Self::Output {
        match expression {
            Expression::Number(n) => {
                self.visit_number(*n)
            }
            Expression::Boolean(b) => {
                self.visit_boolean(*b)
            }
            Expression::BinaryOp { left, op: _, right } => {
                self.visit_binary_op(left, right)
            }
            Expression::UnaryOp { left, op: _ } => {
                self.visit_unary_op(left)
            }
            Expression::Call { identifier: _, args } => {
                self.visit_call(args)
            }
            Expression::Variable(_) => {
                self.visit_variable_expr()
            }
        }
    }

    fn visit_number(&mut self, _n: f64) -> Self::Output {
        // Default: do nothing, numbers are leaves
        Self::Output::default()
    }

    fn visit_boolean(&mut self, _b: bool) -> Self::Output {
        // Default: do nothing, booleans are leaves
        Self::Output::default()
    }

    fn visit_binary_op(&mut self, left: &mut Expression, right: &mut Expression) -> Self::Output {
        self.visit_expression(left);
        self.visit_expression(right);
        Self::Output::default()
    }

    fn visit_unary_op(&mut self, operand: &mut Expression) -> Self::Output {
        self.visit_expression(operand);
        Self::Output::default()
    }

    fn visit_call(&mut self, args: &mut [Expression]) -> Self::Output {
        for arg in args.iter_mut() {
            self.visit_expression(arg);
        }
        Self::Output::default()
    }

    fn visit_variable_expr(&mut self) -> Self::Output {
        // Default: do nothing, variable references are leaves
        Self::Output::default()
    }
}
