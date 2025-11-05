use crate::ast::{Expression, Program, Statement};
use crate::types::Function;
use crate::visitor::{DiagnosticCollector, Visitor};

/// Visitor that performs basic const-folding on the AST
pub struct ASTConstFoldingPass {
    diagnostics: DiagnosticCollector,
}

impl ASTConstFoldingPass {
    pub fn new() -> Self {
        ASTConstFoldingPass {
            diagnostics: DiagnosticCollector::new(),
        }
    }

    fn eval_binop(&self, left: f64, right: f64, op: &crate::lexer::Token) -> Option<f64> {
        use crate::lexer::TokenType;

        match op.tag {
            TokenType::Plus => Some(left + right),
            TokenType::Minus => Some(left - right),
            TokenType::Star => Some(left * right),
            TokenType::Slash => {
                if right == 0.0 {
                    None // Can't fold division by zero
                } else {
                    Some(left / right)
                }
            }
            TokenType::Percent => {
                if right == 0.0 {
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
}

impl Visitor for ASTConstFoldingPass {
    type Output = ();

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_program(&mut self, program: &mut Program) {
        self.walk_program(program);
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
                // Check if both operands are constants
                if let (Expression::Number(a), Expression::Number(b)) = (&**left, &**right) {
                    if let Some(result) = self.eval_binop(*a, *b, op) {
                        self.diagnostics.info(format!(
                            "Const folded {} {} {} to {}",
                            a, op.lexeme, b, result
                        ));
                        *expression = Expression::Number(result);
                    }
                }
            }
            Expression::UnaryOp { left, op } => {
                // Check if operand is a constant
                if let Expression::Number(n) = &**left {
                    if let Some(result) = self.eval_unary(*n, op) {
                        self.diagnostics.info(format!(
                            "Const folded unary {}{} to {}",
                            op.lexeme, n, result
                        ));
                        *expression = Expression::Number(result);
                    }
                }
            }
            _ => {}
        }
    }
}
