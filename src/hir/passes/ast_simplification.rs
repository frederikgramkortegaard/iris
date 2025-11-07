use crate::ast::{Expression, Program, Statement};
use crate::frontend::{Token, TokenType};
use crate::types::Function;
use crate::hir::visitor::{DiagnosticCollector, Visitor};

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

    fn eval_binop(&mut self, left: f64, right: f64, op: &Token) -> Option<f64> {
        use TokenType;

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

    fn eval_unary(&self, operand: f64, op: &Token) -> Option<f64> {
        use TokenType;

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
        op: &Token,
    ) -> Option<bool> {
        use TokenType;

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
        op: &Token,
    ) -> Option<bool> {
        use TokenType;

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

    fn eval_unary_bool(&self, operand: bool, op: &Token) -> Option<bool> {
        use TokenType;

        match op.tag {
            TokenType::Bang => Some(!operand),
            _ => None,
        }
    }

    fn try_algebraic_simplify(&mut self, expression: &mut Expression) {
        // Save type before pattern matching (to avoid borrow issues)
        let saved_typ = expression.typ().clone();

        if let Expression::BinaryOp { left, op, right, span, .. } = expression {
            use TokenType;

            // Normalize commutative operations: put constants on the right
            // This reduces pattern matching cases by half
            let is_commutative = matches!(
                op.tag,
                TokenType::Plus | TokenType::Star | TokenType::And | TokenType::Or | TokenType::Equal | TokenType::NotEqual
            );

            if is_commutative {
                let left_is_const = matches!(left.as_ref(), Expression::Number { .. } | Expression::Boolean { .. });
                let right_is_const = matches!(right.as_ref(), Expression::Number { .. } | Expression::Boolean { .. });

                // If left is constant but right isn't, swap them
                if left_is_const && !right_is_const {
                    std::mem::swap(left, right);
                }
            }

            // Check for variable identities (x op x)
            if let (Expression::Variable { name: a, .. }, Expression::Variable { name: b, .. }) =
                (left.as_ref(), right.as_ref())
            {
                if a == b {
                    let expr_span = *span;
                    let expr_typ = saved_typ.clone();
                    match op.tag {
                        TokenType::Minus => {
                            self.diagnostics.info(format!(
                                "Algebraic simplification: {} - {} -> 0 at line {}, column {}",
                                a, a, op.row, op.column
                            ));
                            *expression = Expression::Number { value: 0.0, span: expr_span, typ: expr_typ };
                            self.folded_nodes_count += 1;
                            return;
                        }
                        TokenType::Equal => {
                            self.diagnostics.info(format!(
                                "Algebraic simplification: {} == {} -> true at line {}, column {}",
                                a, a, op.row, op.column
                            ));
                            *expression = Expression::Boolean { value: true, span: expr_span, typ: expr_typ };
                            self.folded_nodes_count += 1;
                            return;
                        }
                        TokenType::NotEqual => {
                            self.diagnostics.info(format!(
                                "Algebraic simplification: {} != {} -> false at line {}, column {}",
                                a, a, op.row, op.column
                            ));
                            *expression = Expression::Boolean { value: false, span: expr_span, typ: expr_typ };
                            self.folded_nodes_count += 1;
                            return;
                        }
                        TokenType::Less | TokenType::Greater => {
                            self.diagnostics.info(format!(
                                "Algebraic simplification: {} {} {} -> false at line {}, column {}",
                                a, op.lexeme, a, op.row, op.column
                            ));
                            *expression = Expression::Boolean { value: false, span: expr_span, typ: expr_typ };
                            self.folded_nodes_count += 1;
                            return;
                        }
                        TokenType::LessEqual | TokenType::GreaterEqual => {
                            self.diagnostics.info(format!(
                                "Algebraic simplification: {} {} {} -> true at line {}, column {}",
                                a, op.lexeme, a, op.row, op.column
                            ));
                            *expression = Expression::Boolean { value: true, span: expr_span, typ: expr_typ };
                            self.folded_nodes_count += 1;
                            return;
                        }
                        _ => {}
                    }
                }
            }

            // Number identity simplifications
            // (After normalization, constants are always on the right for commutative ops)
            let expr_span = *span;
            let expr_typ = saved_typ.clone();
            match (left.as_ref(), &op.tag, right.as_ref()) {
                // x + 0 -> x
                (_, TokenType::Plus, Expression::Number { value: n, .. }) if *n == 0.0 => {
                    self.diagnostics.info(format!(
                        "Algebraic simplification: expr + 0 -> expr at line {}, column {}",
                        op.row, op.column
                    ));
                    *expression = (**left).clone();
                    self.folded_nodes_count += 1;
                }
                // x - 0 -> x
                (_, TokenType::Minus, Expression::Number { value: n, .. }) if *n == 0.0 => {
                    self.diagnostics.info(format!(
                        "Algebraic simplification: expr - 0 -> expr at line {}, column {}",
                        op.row, op.column
                    ));
                    *expression = (**left).clone();
                    self.folded_nodes_count += 1;
                }
                // x * 1 -> x
                (_, TokenType::Star, Expression::Number { value: n, .. }) if *n == 1.0 => {
                    self.diagnostics.info(format!(
                        "Algebraic simplification: expr * 1 -> expr at line {}, column {}",
                        op.row, op.column
                    ));
                    *expression = (**left).clone();
                    self.folded_nodes_count += 1;
                }
                // x * 0 -> 0
                (_, TokenType::Star, Expression::Number { value: n, .. }) if *n == 0.0 => {
                    self.diagnostics.info(format!(
                        "Algebraic simplification: expr * 0 -> 0 at line {}, column {}",
                        op.row, op.column
                    ));
                    *expression = Expression::Number { value: 0.0, span: expr_span, typ: expr_typ };
                    self.folded_nodes_count += 1;
                }
                // x / 1 -> x
                (_, TokenType::Slash, Expression::Number { value: n, .. }) if *n == 1.0 => {
                    self.diagnostics.info(format!(
                        "Algebraic simplification: expr / 1 -> expr at line {}, column {}",
                        op.row, op.column
                    ));
                    *expression = (**left).clone();
                    self.folded_nodes_count += 1;
                }

                // Boolean identity simplifications
                // x && true -> x
                (_, TokenType::And, Expression::Boolean { value: b, .. }) if *b => {
                    self.diagnostics.info(format!(
                        "Algebraic simplification: expr && true -> expr at line {}, column {}",
                        op.row, op.column
                    ));
                    *expression = (**left).clone();
                    self.folded_nodes_count += 1;
                }
                // x && false -> false
                (_, TokenType::And, Expression::Boolean { value: b, .. }) if !*b => {
                    self.diagnostics.info(format!(
                        "Algebraic simplification: expr && false -> false at line {}, column {}",
                        op.row, op.column
                    ));
                    *expression = Expression::Boolean { value: false, span: expr_span, typ: expr_typ };
                    self.folded_nodes_count += 1;
                }
                // x || true -> true
                (_, TokenType::Or, Expression::Boolean { value: b, .. }) if *b => {
                    self.diagnostics.info(format!(
                        "Algebraic simplification: expr || true -> true at line {}, column {}",
                        op.row, op.column
                    ));
                    *expression = Expression::Boolean { value: true, span: expr_span, typ: expr_typ };
                    self.folded_nodes_count += 1;
                }
                // x || false -> x
                (_, TokenType::Or, Expression::Boolean { value: b, .. }) if !*b => {
                    self.diagnostics.info(format!(
                        "Algebraic simplification: expr || false -> expr at line {}, column {}",
                        op.row, op.column
                    ));
                    *expression = (**left).clone();
                    self.folded_nodes_count += 1;
                }

                _ => {}
            }
        }

        // Handle double negation: !!x -> x
        if let Expression::UnaryOp { left, op, .. } = expression {
            use TokenType;
            if op.tag == TokenType::Bang {
                if let Expression::UnaryOp {
                    left: inner_left,
                    op: inner_op,
                    ..
                } = left.as_ref()
                {
                    if inner_op.tag == TokenType::Bang {
                        self.diagnostics.info(format!(
                            "Algebraic simplification: !!expr -> expr at line {}, column {}",
                            op.row, op.column
                        ));
                        *expression = (**inner_left).clone();
                        self.folded_nodes_count += 1;
                    }
                }
            }
        }
    }

    fn try_constant_fold(&mut self, expression: &mut Expression) {
        // Save type before pattern matching (to avoid borrow issues)
        let saved_typ = expression.typ().clone();

        match expression {
            Expression::BinaryOp { left, op, right, span, .. } => {
                let expr_span = *span;
                let expr_typ = saved_typ.clone();
                // Match on both operands being the same type
                match (left.as_ref(), right.as_ref()) {
                    // Both are numbers
                    (Expression::Number { value: a, .. }, Expression::Number { value: b, .. }) => {
                        // Try arithmetic operations first
                        if let Some(result) = self.eval_binop(*a, *b, op) {
                            self.diagnostics.info(format!(
                                "Const folded {} {} {} to {}",
                                a, op.lexeme, b, result
                            ));
                            *expression = Expression::Number { value: result, span: expr_span, typ: expr_typ };
                            self.folded_nodes_count += 1;
                        }
                        // Try comparison operations (returns bool)
                        else if let Some(result) = self.eval_binop_to_bool_number(*a, *b, op) {
                            self.diagnostics.info(format!(
                                "Const folded {} {} {} to {}",
                                a, op.lexeme, b, result
                            ));
                            *expression = Expression::Boolean { value: result, span: expr_span, typ: expr_typ };
                            self.folded_nodes_count += 1;
                        }
                    }

                    // Both are booleans - logical operations
                    (Expression::Boolean { value: a, .. }, Expression::Boolean { value: b, .. }) => {
                        if let Some(result) = self.eval_binop_to_bool_bool(*a, *b, op) {
                            self.diagnostics.info(format!(
                                "Const folded {} {} {} to {}",
                                a, op.lexeme, b, result
                            ));
                            *expression = Expression::Boolean { value: result, span: expr_span, typ: expr_typ };
                            self.folded_nodes_count += 1;
                        }
                    }

                    _ => {}
                }
            }
            Expression::UnaryOp { left, op, span, .. } => {
                let expr_span = *span;
                let expr_typ = saved_typ.clone();
                match left.as_ref() {
                    Expression::Number { value: n, .. } => {
                        if let Some(result) = self.eval_unary(*n, op) {
                            self.diagnostics.info(format!(
                                "Const folded unary {}{} to {}",
                                op.lexeme, n, result
                            ));
                            *expression = Expression::Number { value: result, span: expr_span, typ: expr_typ };
                            self.folded_nodes_count += 1;
                        }
                    }
                    Expression::Boolean { value: b, .. } => {
                        if let Some(result) = self.eval_unary_bool(*b, op) {
                            self.diagnostics.info(format!(
                                "Const folded unary {}{} to {}",
                                op.lexeme, b, result
                            ));
                            *expression = Expression::Boolean { value: result, span: expr_span, typ: expr_typ };
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

        // Try constant folding
        self.try_constant_fold(expression);

        // After constant folding, try algebraic simplification
        self.try_algebraic_simplify(expression);
    }
}
