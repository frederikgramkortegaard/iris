use crate::frontend::Token;
use crate::span::Span;
use crate::types::{Function, Scope, Type, Variable};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub scope: Option<Rc<RefCell<Scope>>>,
    pub span: Span,
}

impl Block {
    pub fn new(statements: Vec<Statement>, span: Span) -> Self {
        Block {
            statements,
            scope: None,
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Number {
        value: f64,
        span: Span,
    },
    Boolean {
        value: bool,
        span: Span,
    },
    BinaryOp {
        left: Box<Expression>,
        op: Token,
        right: Box<Expression>,
        span: Span,
    },
    UnaryOp {
        left: Box<Expression>,
        op: Token,
        span: Span,
    },
    Call {
        identifier: String, //@TODO : In the future this should be an expression to allow for higher-order functions.
        args: Vec<Expression>,
        span: Span,
    },
    Variable {
        name: String,
        span: Span,
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    Assignment {
        left: String, //@TODO : In the future this should be an expression to allow for assignment into e.g. array indexes
        typ: Option<Type>,
        right: Option<Box<Expression>>,
        span: Span,
    },

    FunctionDefinition {
        name: String,
        args: Vec<Variable>,
        return_type: Type,
        body: Block,
        span: Span,
    },

    If {
        condition: Box<Expression>,
        then: Block,
        els: Option<Block>,
        span: Span,
    },

    While {
        condition: Box<Expression>,
        body: Block,
        span: Span,
    },

    Block {
        block: Block,
        span: Span,
    },

    Return {
        expression: Option<Box<Expression>>,
        span: Span,
    },

    Expression {
        expression: Box<Expression>,
        span: Span,
    },
}

#[derive(Debug)]
pub struct Program {
    pub globals: Vec<Variable>,
    pub functions: Vec<Function>,
}
