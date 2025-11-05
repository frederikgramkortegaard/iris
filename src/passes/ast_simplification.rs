use crate::ast::{Expression, Program, Statement};
use crate::types::Function;
use crate::visitor::{DiagnosticCollector, Visitor};

/// Visitor that performs AST simplification (constant folding, boolean folding, algebraic simplification)
pub struct ASTSimplificationPass {
    diagnostics: DiagnosticCollector,
    folded_nodes_count: u64,
}

impl ASTSimplificationPass {
    pub fn new() -> Self {
        ASTSimplificationPass {
            diagnostics: DiagnosticCollector::new(),
            folded_nodes_count: 0,
        }
    }

    fn eval_binop(&mut self, left: f64, right: f64, op: &crate::lexer::Token) -> Option<f64> {
        use crate::lexer::TokenType;

        match op.tag {
            TokenType::Plus => Some(left + right),
            TokenType::Minus => Some(left - right),
            TokenType::Star => Some(left * right),
            TokenType::Slash => {
                if right == 0.0 {
                    self.diagnostics.warn(format!(
                        "Division by zero: {} / {} at line {}, column {}",
                        left, right, op.row, op.column
                    ));
                    None // Can't fold division by zero
                } else {
                    Some(left / right)
                }
            }
            TokenType::Percent => {
                if right == 0.0 {
                    self.diagnostics.warn(format!(
                        "Modulo by zero: {} % {} at line {}, column {}",
                        left, right, op.row, op.column
                    ));
                    None
                } else {
                    Some(left % right)
                }
            }
            _ => None, // Not a constant-foldable operation (comparisons, logic, etc)
        }
    }

    fn eval_unary(&self, operand: f64, op: &crate::lexer::Token) -> Option<f64> {
        use crate::lexer::TokenType;

        match op.tag {
            TokenType::Minus => Some(-operand),
            TokenType::Plus => Some(operand),
            _ => None, // Not a constant-foldable operation (!, etc)
        }
    }

    fn eval_binop_to_bool_bool(
        &self,
        left: bool,
        right: bool,
        op: &crate::lexer::Token,
    ) -> Option<bool> {
        use crate::lexer::TokenType;

        match op.tag {
            TokenType::And => Some(left && right),
            TokenType::Or => Some(left || right),
            TokenType::Equal => Some(left == right),
            TokenType::NotEqual => Some(left != right),
            _ => None,
        }
    }

    fn eval_binop_to_bool_number(
        &self,
        left: f64,
        right: f64,
        op: &crate::lexer::Token,
    ) -> Option<bool> {
        use crate::lexer::TokenType;

        match op.tag {
            TokenType::Less => Some(left < right),
            TokenType::Greater => Some(left > right),
            TokenType::LessEqual => Some(left <= right),
            TokenType::GreaterEqual => Some(left >= right),
            TokenType::Equal => Some(left == right),
            TokenType::NotEqual => Some(left != right),
            _ => None,
        }
    }

    fn eval_unary_bool(&self, operand: bool, op: &crate::lexer::Token) -> Option<bool> {
        use crate::lexer::TokenType;

        match op.tag {
            TokenType::Bang => Some(!operand),
            _ => None,
        }
    }
}

impl Visitor for ASTSimplificationPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_program(&mut self, program: &mut Program) {
        self.walk_program(program);
        self.diagnostics
            .info(format!("Constant folded {} nodes", self.folded_nodes_count));
    }

    fn visit_function(&mut self, function: &mut Function) {
        self.walk_function(function);
    }

    fn visit_statement(&mut self, statement: &mut Statement) {
        self.walk_statement(statement);
    }

    fn visit_expression(&mut self, expression: &mut Expression) {
        // First fold children (bottom-up)
        self.walk_expression(expression);

        // Then try to fold this expression
        match expression {
            Expression::BinaryOp { left, op, right } => {
                // Match on both operands being the same type
                match (left.as_ref(), right.as_ref()) {
                    // Both are numbers
                    (Expression::Number(a), Expression::Number(b)) => {
                        // Try arithmetic operations first
                        if let Some(result) = self.eval_binop(*a, *b, op) {
                            self.diagnostics.info(format!(
                                "Const folded {} {} {} to {}",
                                a, op.lexeme, b, result
                            ));
                            *expression = Expression::Number(result);
                            self.folded_nodes_count += 1;
                        }
                        // Try comparison operations (returns bool)
                        else if let Some(result) = self.eval_binop_to_bool_number(*a, *b, op) {
                            self.diagnostics.info(format!(
                                "Const folded {} {} {} to {}",
                                a, op.lexeme, b, result
                            ));
                            *expression = Expression::Boolean(result);
                            self.folded_nodes_count += 1;
                        }
                    }

                    // Both are booleans - logical operations
                    (Expression::Boolean(a), Expression::Boolean(b)) => {
                        if let Some(result) = self.eval_binop_to_bool_bool(*a, *b, op) {
                            self.diagnostics.info(format!(
                                "Const folded {} {} {} to {}",
                                a, op.lexeme, b, result
                            ));
                            *expression = Expression::Boolean(result);
                            self.folded_nodes_count += 1;
                        }
                    }

                    // Different types or not constants - no folding
                    _ => {}
                }
            }
            Expression::UnaryOp { left, op } => {
                // Check if operand is a constant

                match left.as_ref() {
                    Expression::Number(n) => {
                        if let Some(result) = self.eval_unary(*n, op) {
                            self.diagnostics.info(format!(
                                "Const folded unary {}{} to {}",
                                op.lexeme, n, result
                            ));
                            *expression = Expression::Number(result);
                            self.folded_nodes_count += 1;
                        }
                    }
                    Expression::Boolean(b) => {
                        if let Some(result) = self.eval_unary_bool(*b, op) {
                            self.diagnostics.info(format!(
                                "Const folded unary {}{} to {}",
                                op.lexeme, b, result
                            ));
                            *expression = Expression::Boolean(result);
                            self.folded_nodes_count += 1;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
