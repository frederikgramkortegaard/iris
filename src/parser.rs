use crate::ast::{Block, Expression, Program, Statement};
use crate::lexer::{Token, TokenType};
use crate::types::{BaseType, Function, Type, Variable};

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

        while self.peek().is_some() && self.peek().unwrap().tag != TokenType::Eof {
            let statement = self.parse_statement()?;
            match statement {
                Statement::Assignment { left, typ, right } => {
                    // If no type specified, default to Auto for type inference
                    let typ = typ.unwrap_or(Type::Base(BaseType::Auto));

                    globals.push(Variable {
                        name: left,
                        typ,
                        initializer: right,
                    });
                }
                Statement::FunctionDefinition {
                    name,
                    args,
                    return_type,
                    body,
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
                    })
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
                    _ => {
                        return Err(ParseError {
                            message: format!("Expected type, got {:?}", token.tag),
                        })
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

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let mut statements = Vec::new();
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
        Ok(Block::new(statements))
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.peek() {
            Some(token) => match token.tag {
                // Error on semicolon - not in the language
                TokenType::Semicolon => {
                    Err(ParseError {
                        message: format!(
                            "Unexpected semicolon at line {}:{}. This language does not use semicolons.",
                            token.row, token.column
                        ),
                    })
                }

                // Function definition
                TokenType::Fn => {
                    self.consume(); // consume 'fn'

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
                    self.consume_assert(
                        TokenType::LBrace,
                        "Expected '{' before function body".to_string(),
                    )?;
                    let body = self.parse_block()?;
                    self.consume_assert(
                        TokenType::RBrace,
                        "Expected '}' after function body".to_string(),
                    )?;

                    Ok(Statement::FunctionDefinition {
                        name: name.lexeme,
                        args,
                        return_type,
                        body,
                    })
                }

                TokenType::LBrace => {
                    self.consume();

                    let body = self.parse_block()?;
                    self.consume_assert(TokenType::RBrace, "Missing } after body".to_string())?;

                    Ok(Statement::Block(body))
                }
                TokenType::Return => {
                    self.consume();
                    // Check if there's an expression after return
                    let expr = match self.peek() {
                        // If we see a closing brace or EOF, it's a bare return
                        Some(t) if t.tag == TokenType::RBrace || t.tag == TokenType::Eof => None,
                        // Otherwise parse the expression
                        Some(_) => Some(Box::new(self.parse_expression()?)),
                        None => None,
                    };
                    Ok(Statement::Return(expr))
                }
                TokenType::While => {
                    self.consume();
                    self.consume_optional(TokenType::LParen);
                    let condition = Box::new(self.parse_expression()?);
                    self.consume_optional(TokenType::RParen);

                    self.consume_assert(
                        TokenType::LBrace,
                        "Missing { after while conditional".to_string(),
                    )?;

                    let body = self.parse_block()?;

                    self.consume_assert(
                        TokenType::RBrace,
                        "Missing } after while body".to_string(),
                    )?;

                    Ok(Statement::While { condition, body })
                }
                TokenType::If => {
                    self.consume();
                    self.consume_optional(TokenType::LParen);
                    let condition = Box::new(self.parse_expression()?);
                    self.consume_optional(TokenType::RParen);

                    self.consume_assert(
                        TokenType::LBrace,
                        "Missing { after if conditional".to_string(),
                    )?;

                    let then = self.parse_block()?;

                    self.consume_assert(TokenType::RBrace, "Missing } after if body".to_string())?;

                    let els = match self.peek() {
                        Some(token) if token.tag == TokenType::Else => {
                            self.consume(); // consume 'else'
                            self.consume_assert(
                                TokenType::LBrace,
                                "Expected '{' after 'else'".to_string(),
                            )?;
                            let block = self.parse_block()?;
                            self.consume_assert(
                                TokenType::RBrace,
                                "Expected '}' after else body".to_string(),
                            )?;
                            Some(block)
                        }
                        _ => None,
                    };

                    Ok(Statement::If {
                        condition,
                        then,
                        els,
                    })
                }

                TokenType::Identifier => {
                    match self.peek_offset(1) {
                        Some(t) if t.tag == TokenType::Assign => {
                            // Assignment: x = ...
                            let identifier = self.consume().unwrap();
                            self.consume(); // consume '='
                            let right = self.parse_expression().ok().map(Box::new);
                            Ok(Statement::Assignment {
                                left: identifier.lexeme,
                                typ: None,
                                right,
                            })
                        }
                        // Expression Statement
                        Some(_) => {
                            let expr = self.parse_expression()?;
                            Ok(Statement::Expression(Box::new(expr)))
                        }
                        None => Err(ParseError {
                            message: "Unexpected end of input".to_string(),
                        }),
                    }
                }

                // Variable Declarations and Assignments
                TokenType::Var => {
                    self.consume();
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

                    Ok(Statement::Assignment {
                        left: identifier.lexeme,
                        typ,
                        right,
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
                    let token = self.consume().unwrap();
                    let value = token.lexeme.parse::<f64>().map_err(|_| ParseError {
                        message: format!("Failed to parse number: {}", token.lexeme),
                    })?;
                    Ok(Expression::Number(value))
                }

                // Identifier or function call
                TokenType::Identifier => {
                    let identifier = self.consume().unwrap();

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

                            self.consume_assert(
                                TokenType::RParen,
                                "Expected ')' after arguments".to_string(),
                            )?;

                            return Ok(Expression::Call {
                                identifier: identifier.lexeme,
                                args,
                            });
                        }
                    }

                    // Just a variable reference
                    Ok(Expression::Variable(identifier.lexeme))
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
                    let op = self.consume().unwrap();
                    let expr = self.parse_unary()?;
                    Ok(Expression::UnaryOp {
                        op,
                        left: Box::new(expr),
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
            let op = self.consume().unwrap();

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
            lhs = Box::new(Expression::BinaryOp {
                left: lhs,
                op,
                right: rhs,
            });
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        let lhs = self.parse_unary()?;
        self.parse_binop_rhs(0, Box::new(lhs)).map(|b| *b)
    }
}
