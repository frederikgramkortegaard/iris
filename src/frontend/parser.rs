use crate::ast::{Block, Expression, Program, Statement};
use crate::frontend::{Token, TokenType};
use crate::span::Span;
use crate::types::{BaseType, Function, Type, Variable};

impl Expression {
    /// Get the span of this expression
    pub fn span(&self) -> Span {
        match self {
            Expression::Number { span, .. } => *span,
            Expression::Boolean { span, .. } => *span,
            Expression::BinaryOp { span, .. } => *span,
            Expression::UnaryOp { span, .. } => *span,
            Expression::Call { span, .. } => *span,
            Expression::Variable { span, .. } => *span,
            Expression::Range { span, .. } => *span,
        }
    }
}

/// Error type returned when parsing fails.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
}

/// The parser context that maintains state during parsing.
pub struct ParserContext {
    tokens: Vec<Token>,
    position: usize,
}

impl ParserContext {
    pub fn new(tokens: Vec<Token>) -> Self {
        ParserContext {
            tokens,
            position: 0,
        }
    }

    fn get_precedence(&self, token_type: &TokenType) -> i8 {
        match token_type {
            TokenType::DoubleDots => 3,
            TokenType::Or => 5,
            TokenType::And => 6,
            TokenType::Equal | TokenType::NotEqual => 10,
            TokenType::Less
            | TokenType::Greater
            | TokenType::LessEqual
            | TokenType::GreaterEqual => 10,
            TokenType::Plus | TokenType::Minus => 20,
            TokenType::Star | TokenType::Slash | TokenType::Percent => 40,
            _ => -1, // Not a binary operator
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.peek_offset(0)
    }

    fn peek_offset(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.position + offset)
    }

    fn consume(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position)?.clone();
        self.position += 1;
        Some(token)
    }

    fn consume_optional(&mut self, expected_type: TokenType) -> Option<Token> {
        match self.peek() {
            Some(token) if token.tag == expected_type => self.consume(),
            _ => None,
        }
    }

    fn consume_assert(
        &mut self,
        expected_type: TokenType,
        message: String,
    ) -> Result<Token, ParseError> {
        match self.consume() {
            Some(tok) if tok.tag == expected_type => Ok(tok),
            Some(tok) => Err(ParseError {
                message: format!(
                    "{} at {}:{} (got {:?})",
                    message, tok.row, tok.column, tok.tag
                ),
            }),
            None => Err(ParseError {
                message: format!("{} (unexpected end of input)", message),
            }),
        }
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut globals: Vec<Variable> = Vec::new();
        let mut functions: Vec<Function> = Vec::new();

        while self.peek().is_some_and(|t| t.tag != TokenType::Eof) {
            let statement = self.parse_statement()?;
            match statement {
                Statement::Assignment {
                    left, typ, right, ..
                } => {
                    // If no type specified, default to Auto for type inference
                    let typ = typ.unwrap_or(Type::Base(BaseType::Auto));

                    globals.push(Variable {
                        name: left,
                        read_only: false,
                        typ,
                        initializer: right,
                    });
                }
                Statement::FunctionDefinition {
                    name,
                    args,
                    return_type,
                    body,
                    ..
                } => {
                    functions.push(Function {
                        name,
                        args,
                        return_type,
                        body,
                    });
                }
                _ => {
                    return Err(ParseError {
                        message: format!(
                            "Unexpected statement at top level: {:?}. Only function definitions and variable declarations are allowed at the top level.",
                            statement
                        ),
                    });
                }
            }
        }

        Ok(Program { globals, functions })
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        // Check for pointer prefix (*)
        if let Some(token) = self.peek() {
            if token.tag == TokenType::Star {
                self.consume(); // consume '*'
                let inner_type = self.parse_type()?;
                return Ok(Type::PointerType(Box::new(inner_type)));
            }
        }

        // Parse base type
        match self.peek() {
            Some(token) => {
                let base_type = match token.tag {
                    TokenType::F8Type => BaseType::F8,
                    TokenType::F16Type => BaseType::F16,
                    TokenType::F32Type => BaseType::F32,
                    TokenType::F64Type => BaseType::F64,
                    TokenType::BoolType => BaseType::Bool,
                    _ => {
                        return Err(ParseError {
                            message: format!("Expected type, got {:?}", token.tag),
                        });
                    }
                };
                self.consume(); // consume the type token
                Ok(Type::Base(base_type))
            }
            None => Err(ParseError {
                message: "Expected type, got end of input".to_string(),
            }),
        }
    }

    fn parse_block(&mut self, start_token: &Token) -> Result<Block, ParseError> {
        let mut statements = Vec::new();
        let start_span = Span::from_token(start_token);

        while self.peek().is_some() {
            // Stop when we hit a closing brace
            if let Some(token) = self.peek() {
                if token.tag == TokenType::RBrace {
                    break;
                }
            }
            let statement = self.parse_statement()?;
            statements.push(statement);
        }

        // Compute the span: from start_token to the last statement (or just start_token if empty)
        let span = if let Some(last_stmt) = statements.last() {
            let end_span = match last_stmt {
                Statement::Assignment { span, .. } => *span,
                Statement::FunctionDefinition { span, .. } => *span,
                Statement::If { span, .. } => *span,
                Statement::While { span, .. } => *span,
                Statement::Block { span, .. } => *span,
                Statement::Return { span, .. } => *span,
                Statement::Expression { span, .. } => *span,
                Statement::For { span, .. } => *span,
            };
            Span::merge(&start_span, &end_span)
        } else {
            start_span
        };

        Ok(Block::new(statements, span))
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.peek() {
            Some(token) => match token.tag {
                // Error on semicolon - not in the language
                TokenType::Semicolon => Err(ParseError {
                    message: format!(
                        "Unexpected semicolon at line {}:{}. This language does not use semicolons.",
                        token.row, token.column
                    ),
                }),

                // Function definition
                TokenType::Fn => {
                    let fn_token = self.consume().expect("token verified by peek"); // consume 'fn'

                    let name = self.consume_assert(
                        TokenType::Identifier,
                        "Expected function name after 'fn'".to_string(),
                    )?;

                    self.consume_assert(
                        TokenType::LParen,
                        "Expected '(' after function name".to_string(),
                    )?;

                    // Parse argument list
                    let mut args: Vec<Variable> = Vec::new();
                    while let Some(t) = self.peek() {
                        if t.tag == TokenType::RParen {
                            break;
                        }

                        // Parse argument: name: type [= default_value]
                        let arg_name = self.consume_assert(
                            TokenType::Identifier,
                            "Expected argument name".to_string(),
                        )?;

                        self.consume_assert(
                            TokenType::Colon,
                            "Expected ':' after argument name".to_string(),
                        )?;

                        let arg_type = self.parse_type()?;

                        // Check for default value
                        let initializer = if let Some(t) = self.peek() {
                            if t.tag == TokenType::Assign {
                                self.consume(); // consume '='
                                Some(Box::new(self.parse_expression()?))
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        args.push(Variable {
                            name: arg_name.lexeme,
                            read_only: false,
                            typ: arg_type,
                            initializer,
                        });

                        // Check for comma or end of args
                        if let Some(t) = self.peek() {
                            if t.tag == TokenType::Comma {
                                self.consume();
                            }
                        }
                    }

                    self.consume_assert(
                        TokenType::RParen,
                        "Expected ')' after arguments".to_string(),
                    )?;

                    // Parse return type (optional, defaults to void)
                    let return_type = if self.consume_optional(TokenType::Arrow).is_some() {
                        self.parse_type()?
                    } else {
                        Type::Base(BaseType::Void)
                    };

                    // Parse body
                    let lbrace = self.consume_assert(
                        TokenType::LBrace,
                        "Expected '{' before function body".to_string(),
                    )?;
                    let body = self.parse_block(&lbrace)?;
                    let rbrace = self.consume_assert(
                        TokenType::RBrace,
                        "Expected '}' after function body".to_string(),
                    )?;

                    let span =
                        Span::merge(&Span::from_token(&fn_token), &Span::from_token(&rbrace));

                    Ok(Statement::FunctionDefinition {
                        name: name.lexeme,
                        args,
                        return_type,
                        body,
                        span,
                    })
                }

                TokenType::LBrace => {
                    let lbrace = self.consume().expect("token verified by peek");

                    let body = self.parse_block(&lbrace)?;
                    let rbrace =
                        self.consume_assert(TokenType::RBrace, "Missing } after body".to_string())?;

                    let span = Span::merge(&Span::from_token(&lbrace), &Span::from_token(&rbrace));

                    Ok(Statement::Block { block: body, span })
                }
                TokenType::Return => {
                    let return_token = self.consume().expect("token verified by peek");
                    // Check if there's an expression after return
                    let expr = match self.peek() {
                        // If we see a closing brace or EOF, it's a bare return
                        Some(t) if t.tag == TokenType::RBrace || t.tag == TokenType::Eof => None,
                        // Otherwise parse the expression
                        Some(_) => Some(Box::new(self.parse_expression()?)),
                        None => None,
                    };

                    let span = if let Some(e) = &expr {
                        Span::merge(&Span::from_token(&return_token), &e.span())
                    } else {
                        Span::from_token(&return_token)
                    };

                    Ok(Statement::Return {
                        expression: expr,
                        span,
                    })
                }
                TokenType::For => {
                    let for_token = self.consume().expect("token verified by peek");

                    // Check for "for ident in ....", basically a for loop where we catch the value
                    // of enumeration

                    let identifier = if self.peek().is_some_and(|t| t.tag == TokenType::Identifier) {
                        let token = self.consume().expect("token verified by peek");
                        self.consume_assert(TokenType::In, "Expected 'in' after identifier in for loop".to_string())?;
                        Some(token.lexeme)
                    } else {
                        None
                    };

                    let range = Box::new(self.parse_expression().expect("No range was defined in the for loop"));

                    let lbrace = self.consume_assert(
                        TokenType::LBrace,
                        "Missing { after while conditional".to_string(),
                    )?;

                    let body = self.parse_block(&lbrace)?;

                    let rbrace = self.consume_assert(
                        TokenType::RBrace,
                        "Missing } after while body".to_string(),
                    )?;

                    let span =
                        Span::merge(&Span::from_token(&for_token), &Span::from_token(&rbrace));


                    Ok(Statement::For {
                        ident: identifier,
                        range,
                        body,
                        span,
                    })

                }
                TokenType::While => {
                    let while_token = self.consume().expect("token verified by peek");
                    self.consume_optional(TokenType::LParen);
                    let condition = Box::new(self.parse_expression()?);
                    self.consume_optional(TokenType::RParen);

                    let lbrace = self.consume_assert(
                        TokenType::LBrace,
                        "Missing { after while conditional".to_string(),
                    )?;

                    let body = self.parse_block(&lbrace)?;

                    let rbrace = self.consume_assert(
                        TokenType::RBrace,
                        "Missing } after while body".to_string(),
                    )?;

                    let span =
                        Span::merge(&Span::from_token(&while_token), &Span::from_token(&rbrace));

                    Ok(Statement::While {
                        condition,
                        body,
                        span,
                    })
                }
                TokenType::If => {
                    let if_token = self.consume().expect("token verified by peek");
                    self.consume_optional(TokenType::LParen);
                    let condition = Box::new(self.parse_expression()?);
                    self.consume_optional(TokenType::RParen);

                    let lbrace = self.consume_assert(
                        TokenType::LBrace,
                        "Missing { after if conditional".to_string(),
                    )?;

                    let then = self.parse_block(&lbrace)?;

                    let mut rbrace = self
                        .consume_assert(TokenType::RBrace, "Missing } after if body".to_string())?;

                    let els = match self.peek() {
                        Some(token) if token.tag == TokenType::Else => {
                            self.consume(); // consume 'else'
                            let else_lbrace = self.consume_assert(
                                TokenType::LBrace,
                                "Expected '{' after 'else'".to_string(),
                            )?;
                            let block = self.parse_block(&else_lbrace)?;
                            rbrace = self.consume_assert(
                                TokenType::RBrace,
                                "Expected '}' after else body".to_string(),
                            )?;
                            Some(block)
                        }
                        _ => None,
                    };

                    let span =
                        Span::merge(&Span::from_token(&if_token), &Span::from_token(&rbrace));

                    Ok(Statement::If {
                        condition,
                        then,
                        els,
                        span,
                    })
                }

                TokenType::Identifier => {
                    match self.peek_offset(1) {
                        Some(t) if t.tag == TokenType::Assign => {
                            // Assignment: x = ...
                            let identifier = self.consume().expect("token verified by peek");
                            self.consume(); // consume '='
                            let right = self.parse_expression().ok().map(Box::new);

                            let span = if let Some(r) = &right {
                                Span::merge(&Span::from_token(&identifier), &r.span())
                            } else {
                                Span::from_token(&identifier)
                            };

                            Ok(Statement::Assignment {
                                left: identifier.lexeme,
                                typ: None,
                                right,
                                span,
                            })
                        }
                        // Expression Statement
                        Some(_) => {
                            let expr = self.parse_expression()?;
                            let span = expr.span();
                            Ok(Statement::Expression {
                                expression: Box::new(expr),
                                span,
                            })
                        }
                        None => Err(ParseError {
                            message: "Unexpected end of input".to_string(),
                        }),
                    }
                }

                // Variable Declarations and Assignments
                TokenType::Var => {
                    let var_token = self.consume().expect("token verified by peek");
                    let identifier = self.consume_assert(
                        TokenType::Identifier,
                        "Expected an identifier after 'var'".to_string(),
                    )?;

                    let typ = match self.peek() {
                        Some(t) if t.tag == TokenType::Colon => {
                            // Has explicit type annotation: var x: f64
                            self.consume(); // consume ':'
                            Some(self.parse_type()?)
                        }
                        _ => {
                            // No explicit type, default to Auto for type inference
                            Some(Type::Base(BaseType::Auto))
                        }
                    };

                    let right = match self.peek() {
                        Some(t) if t.tag == TokenType::Assign => {
                            self.consume();
                            self.parse_expression().ok().map(Box::new)
                        }
                        _ => None,
                    };

                    let span = if let Some(r) = &right {
                        Span::merge(&Span::from_token(&var_token), &r.span())
                    } else {
                        Span::merge(
                            &Span::from_token(&var_token),
                            &Span::from_token(&identifier),
                        )
                    };

                    Ok(Statement::Assignment {
                        left: identifier.lexeme,
                        typ,
                        right,
                        span,
                    })
                }

                _ => Err(ParseError {
                    message: format!("Unexpected token: {:?}", token.tag),
                }),
            },
            None => Err(ParseError {
                message: "Unexpected end of input".to_string(),
            }),
        }
    }

    // Parse primary expressions - numbers, identifiers, function calls, parenthesized expressions
    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        match self.peek() {
            Some(token) => match token.tag {
                // Parenthesized expression
                TokenType::LParen => {
                    self.consume(); // consume '('
                    let expr = self.parse_expression()?;
                    self.consume_assert(
                        TokenType::RParen,
                        "Expected ')' after expression".to_string(),
                    )?;
                    Ok(expr)
                }

                // Number literal
                TokenType::Number => {
                    let token = self.consume().expect("token verified by peek");
                    let value = token.lexeme.parse::<f64>().map_err(|_| ParseError {
                        message: format!("Failed to parse number: {}", token.lexeme),
                    })?;
                    Ok(Expression::Number {
                        value,
                        span: Span::from_token(&token),
                        typ: None,
                    })
                }

                // Boolean literals
                TokenType::True => {
                    let token = self.consume().expect("token verified by peek");
                    Ok(Expression::Boolean {
                        value: true,
                        span: Span::from_token(&token),
                        typ: None,
                    })
                }
                TokenType::False => {
                    let token = self.consume().expect("token verified by peek");
                    Ok(Expression::Boolean {
                        value: false,
                        span: Span::from_token(&token),
                        typ: None,
                    })
                }

                // Identifier or function call
                TokenType::Identifier => {
                    let identifier = self.consume().expect("token verified by peek");

                    // Check if it's a function call
                    if let Some(t) = self.peek() {
                        if t.tag == TokenType::LParen {
                            self.consume(); // consume '('

                            let mut args = Vec::new();

                            // Parse arguments if present
                            if let Some(t) = self.peek() {
                                if t.tag != TokenType::RParen {
                                    args.push(self.parse_expression()?);

                                    while let Some(t) = self.peek() {
                                        if t.tag == TokenType::Comma {
                                            self.consume(); // consume ','
                                            args.push(self.parse_expression()?);
                                        } else {
                                            break;
                                        }
                                    }
                                }
                            }

                            let rparen = self.consume_assert(
                                TokenType::RParen,
                                "Expected ')' after arguments".to_string(),
                            )?;

                            let span = Span::merge(
                                &Span::from_token(&identifier),
                                &Span::from_token(&rparen),
                            );

                            return Ok(Expression::Call {
                                identifier: identifier.lexeme,
                                args,
                                span,
                                typ: None,
                            });
                        }
                    }

                    // Just a variable reference
                    let span = Span::from_token(&identifier);
                    Ok(Expression::Variable {
                        name: identifier.lexeme,
                        span,
                        typ: None,
                    })
                }

                _ => Err(ParseError {
                    message: format!("Unexpected token in expression: {:?}", token.tag),
                }),
            },
            None => Err(ParseError {
                message: "Unexpected end of input in expression".to_string(),
            }),
        }
    }

    // Parse unary expressions
    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        match self.peek() {
            Some(token) => match token.tag {
                TokenType::Plus | TokenType::Minus | TokenType::Bang => {
                    let op = self.consume().expect("token verified by peek");
                    let expr = self.parse_unary()?;
                    let span = Span::merge(&Span::from_token(&op), &expr.span());
                    Ok(Expression::UnaryOp {
                        op,
                        left: Box::new(expr),
                        span,
                        typ: None,
                    })
                }
                _ => self.parse_primary(),
            },
            None => Err(ParseError {
                message: "Unexpected end of input in expression".to_string(),
            }),
        }
    }

    // Parse the RHS of a binary expression using precedence climbing
    fn parse_binop_rhs(
        &mut self,
        expr_prec: i8,
        mut lhs: Box<Expression>,
    ) -> Result<Box<Expression>, ParseError> {
        loop {
            // Get the precedence of the next operator
            let tok_prec = match self.peek() {
                Some(token) => self.get_precedence(&token.tag),
                None => return Ok(lhs),
            };

            // If this operator binds less tightly than the current expression, return LHS
            if tok_prec < expr_prec {
                return Ok(lhs);
            }

            // Consume the operator
            let op = self.consume().expect("token verified by peek");

            // Parse the primary expression after the binary operator
            let mut rhs = Box::new(self.parse_unary()?);

            // Check the next operator's precedence for right-associativity
            let next_prec = match self.peek() {
                Some(token) => self.get_precedence(&token.tag),
                None => -1,
            };

            if tok_prec < next_prec {
                rhs = self.parse_binop_rhs(tok_prec + 1, rhs)?;
            }

            // Merge LHS and RHS
            let span = Span::merge(&lhs.span(), &rhs.span());
            lhs = match op.tag {
                TokenType::DoubleDots => Box::new(Expression::Range {
                    start: lhs,
                    end: rhs,
                    span,
                    typ: None,
                }),
                _ => Box::new(Expression::BinaryOp {
                    left: lhs,
                    op,
                    right: rhs,
                    span,
                    typ: None,
                }),
            };
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        let lhs = self.parse_unary()?;
        self.parse_binop_rhs(0, Box::new(lhs)).map(|b| *b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::LexerContext;

    fn parse(input: &str) -> Program {
        let tokens = LexerContext::lex(input).expect("lexing failed");
        ParserContext::new(tokens).parse().expect("parsing failed")
    }

    fn parse_err(input: &str) -> ParseError {
        let tokens = LexerContext::lex(input).expect("lexing failed");
        ParserContext::new(tokens)
            .parse()
            .expect_err("expected parse error")
    }

    // === Expression Tests ===

    #[test]
    fn number_literal() {
        let prog = parse("fn main() { return 42 }");
        let func = &prog.functions[0];
        if let Statement::Return {
            expression: Some(expr),
            ..
        } = &func.body.statements[0]
        {
            assert!(matches!(expr.as_ref(), Expression::Number { value, .. } if *value == 42.0));
        } else {
            panic!("expected return statement");
        }
    }

    #[test]
    fn boolean_literals() {
        let prog = parse("fn main() { return true }");
        let func = &prog.functions[0];
        if let Statement::Return {
            expression: Some(expr),
            ..
        } = &func.body.statements[0]
        {
            assert!(matches!(
                expr.as_ref(),
                Expression::Boolean { value: true, .. }
            ));
        } else {
            panic!("expected return statement");
        }
    }

    #[test]
    fn binary_operations() {
        let prog = parse("fn main() { return 1 + 2 * 3 }");
        let func = &prog.functions[0];
        if let Statement::Return {
            expression: Some(expr),
            ..
        } = &func.body.statements[0]
        {
            // Should parse as 1 + (2 * 3) due to precedence
            if let Expression::BinaryOp {
                left, op, right, ..
            } = expr.as_ref()
            {
                assert_eq!(op.tag, TokenType::Plus);
                assert!(matches!(left.as_ref(), Expression::Number { value, .. } if *value == 1.0));
                assert!(matches!(right.as_ref(), Expression::BinaryOp { .. }));
            } else {
                panic!("expected binary op");
            }
        } else {
            panic!("expected return statement");
        }
    }

    #[test]
    fn unary_operations() {
        let prog = parse("fn main() { return -5 }");
        let func = &prog.functions[0];
        if let Statement::Return {
            expression: Some(expr),
            ..
        } = &func.body.statements[0]
        {
            if let Expression::UnaryOp { op, left, .. } = expr.as_ref() {
                assert_eq!(op.tag, TokenType::Minus);
                assert!(matches!(left.as_ref(), Expression::Number { value, .. } if *value == 5.0));
            } else {
                panic!("expected unary op");
            }
        } else {
            panic!("expected return statement");
        }
    }

    #[test]
    fn function_call() {
        let prog = parse("fn main() { return foo(1, 2) }");
        let func = &prog.functions[0];
        if let Statement::Return {
            expression: Some(expr),
            ..
        } = &func.body.statements[0]
        {
            if let Expression::Call {
                identifier, args, ..
            } = expr.as_ref()
            {
                assert_eq!(identifier, "foo");
                assert_eq!(args.len(), 2);
            } else {
                panic!("expected call");
            }
        } else {
            panic!("expected return statement");
        }
    }

    #[test]
    fn parenthesized_expression() {
        let prog = parse("fn main() { return (1 + 2) * 3 }");
        let func = &prog.functions[0];
        if let Statement::Return {
            expression: Some(expr),
            ..
        } = &func.body.statements[0]
        {
            // Should parse as (1 + 2) * 3
            if let Expression::BinaryOp { op, .. } = expr.as_ref() {
                assert_eq!(op.tag, TokenType::Star);
            } else {
                panic!("expected binary op");
            }
        } else {
            panic!("expected return statement");
        }
    }

    // === Statement Tests ===

    #[test]
    fn function_definition() {
        let prog = parse("fn add(x: f64, y: f64) -> f64 { return x + y }");
        assert_eq!(prog.functions.len(), 1);
        let func = &prog.functions[0];
        assert_eq!(func.name, "add");
        assert_eq!(func.args.len(), 2);
        assert_eq!(func.args[0].name, "x");
        assert_eq!(func.args[1].name, "y");
    }

    #[test]
    fn function_no_return_type() {
        let prog = parse("fn main() { }");
        let func = &prog.functions[0];
        assert!(matches!(func.return_type, Type::Base(BaseType::Void)));
    }

    #[test]
    fn variable_declaration() {
        let prog = parse("var x: f64 = 5");
        assert_eq!(prog.globals.len(), 1);
        assert_eq!(prog.globals[0].name, "x");
    }

    #[test]
    fn variable_declaration_inferred() {
        let prog = parse("var x = 5");
        assert_eq!(prog.globals.len(), 1);
        assert!(matches!(prog.globals[0].typ, Type::Base(BaseType::Auto)));
    }

    #[test]
    fn if_statement() {
        let prog = parse("fn main() { if true { return 1 } }");
        let func = &prog.functions[0];
        assert!(matches!(
            &func.body.statements[0],
            Statement::If { els: None, .. }
        ));
    }

    #[test]
    fn if_else_statement() {
        let prog = parse("fn main() { if true { return 1 } else { return 2 } }");
        let func = &prog.functions[0];
        assert!(matches!(
            &func.body.statements[0],
            Statement::If { els: Some(_), .. }
        ));
    }

    #[test]
    fn while_statement() {
        let prog = parse("fn main() { while true { return 1 } }");
        let func = &prog.functions[0];
        assert!(matches!(&func.body.statements[0], Statement::While { .. }));
    }

    #[test]
    fn assignment() {
        let prog = parse("fn main() { x = 5 }");
        let func = &prog.functions[0];
        if let Statement::Assignment { left, .. } = &func.body.statements[0] {
            assert_eq!(left, "x");
        } else {
            panic!("expected assignment");
        }
    }

    // === Type Tests ===

    #[test]
    fn pointer_type() {
        let prog = parse("fn main(x: *f64) { }");
        let func = &prog.functions[0];
        assert!(matches!(&func.args[0].typ, Type::PointerType(_)));
    }

    // === Error Tests ===

    #[test]
    fn error_semicolon() {
        let err = parse_err("fn main() { return 5; }");
        assert!(err.message.contains("semicolon"));
    }

    #[test]
    fn error_missing_brace() {
        let err = parse_err("fn main() { return 5");
        assert!(err.message.contains("Eof"));
    }

    #[test]
    fn error_unexpected_top_level() {
        let err = parse_err("return 5");
        assert!(err.message.contains("top level"));
    }

    // === For Loop Tests ===

    #[test]
    fn for_loop_with_identifier() {
        let prog = parse("fn main() { for i in 0..10 { return i } }");
        let func = &prog.functions[0];
        if let Statement::For { ident, range, .. } = &func.body.statements[0] {
            assert_eq!(ident.as_ref().unwrap(), "i");
            assert!(matches!(range.as_ref(), Expression::Range { .. }));
        } else {
            panic!("expected for statement");
        }
    }

    #[test]
    fn for_loop_without_identifier() {
        let prog = parse("fn main() { for 0..5 { return 1 } }");
        let func = &prog.functions[0];
        if let Statement::For { ident, range, .. } = &func.body.statements[0] {
            assert!(ident.is_none());
            assert!(matches!(range.as_ref(), Expression::Range { .. }));
        } else {
            panic!("expected for statement");
        }
    }

    #[test]
    fn for_loop_range_bounds() {
        let prog = parse("fn main() { for x in 1..100 { return x } }");
        let func = &prog.functions[0];
        if let Statement::For { range, .. } = &func.body.statements[0] {
            if let Expression::Range { start, end, .. } = range.as_ref() {
                assert!(matches!(start.as_ref(), Expression::Number { value, .. } if *value == 1.0));
                assert!(matches!(end.as_ref(), Expression::Number { value, .. } if *value == 100.0));
            } else {
                panic!("expected range expression");
            }
        } else {
            panic!("expected for statement");
        }
    }
}
