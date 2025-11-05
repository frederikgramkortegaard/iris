use crate::lexer::Token;
use crate::types::{Function, Scope, Type, Variable};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub scope: Option<Rc<RefCell<Scope>>>,
}

impl Block {
    pub fn new(statements: Vec<Statement>) -> Self {
        Block {
            statements,
            scope: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Number(f64),
    Boolean(bool),
    BinaryOp {
        left: Box<Expression>,
        op: Token,
        right: Box<Expression>,
    },
    UnaryOp {
        left: Box<Expression>,
        op: Token,
    },
    Call {
        identifier: String, //@TODO : In the future this should be an expression to allow for higher-order functions.
        args: Vec<Expression>,
    },
    Variable(String)
}

#[derive(Debug, Clone)]
pub enum Statement {
    Assignment {
        left: String, //@TODO : In the future this should be an expression to allow for assignment into e.g. array indexes
        typ: Option<Type>,
        right: Option<Box<Expression>>,
    },

    FunctionDefinition {
        name: String,
        args: Vec<Variable>,
        return_type: Type,
        body: Block,
    },

    If {
        condition: Box<Expression>,
        then: Block,
        els: Option<Block>,
    },

    While {
        condition: Box<Expression>,
        body: Block,
    },

    Block(Block),

    Return(Option<Box<Expression>>),

    Expression(Box<Expression>),
}

#[derive(Debug)]
pub struct Program {
    pub globals: Vec<Variable>,
    pub functions: Vec<Function>,
}
