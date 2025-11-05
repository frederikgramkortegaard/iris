use crate::ast::Block;
use crate::lexer::TokenType;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum BaseType {
    F8,
    F16,
    F32,
    F64,
    Bool,
    Void,
    Auto,
}

#[derive(Debug, Clone)]
pub enum Type {
    Base(BaseType),
    PointerType(Box<Type>),
}

impl Type {
    /// Check if two types are compatible (equal or Auto)
    pub fn is_equal(&self, other: &Type) -> bool {
        match (self, other) {
            // Auto is compatible with anything
            (Type::Base(BaseType::Auto), _) => true,
            (_, Type::Base(BaseType::Auto)) => true,
            // Otherwise check exact equality
            (Type::Base(a), Type::Base(b)) => match (a, b) {
                (BaseType::F8, BaseType::F8) => true,
                (BaseType::F16, BaseType::F16) => true,
                (BaseType::F32, BaseType::F32) => true,
                (BaseType::F64, BaseType::F64) => true,
                (BaseType::Bool, BaseType::Bool) => true,
                (BaseType::Void, BaseType::Void) => true,
                _ => false,
            },
            (Type::PointerType(a), Type::PointerType(b)) => a.is_equal(b),
            _ => false,
        }
    }

    /// Check if this type can be used with another in a binary operation
    /// Returns the result type if compatible, None if not
    pub fn binop_result(&self, op: &TokenType, other: &Type) -> Option<Type> {
        // Check if operands are compatible
        if !self.is_equal(other) {
            return None;
        }

        // Determine result type based on operator
        match op {
            // Comparison operators return Bool
            TokenType::Equal
            | TokenType::NotEqual
            | TokenType::Less
            | TokenType::Greater
            | TokenType::LessEqual
            | TokenType::GreaterEqual => Some(Type::Base(BaseType::Bool)),

            // Logical operators return Bool (and require Bool operands)
            TokenType::And | TokenType::Or => {
                if matches!(self, Type::Base(BaseType::Bool)) {
                    Some(Type::Base(BaseType::Bool))
                } else {
                    None // Logical operators require Bool operands
                }
            }

            // Arithmetic operators return the same type as operands
            TokenType::Plus
            | TokenType::Minus
            | TokenType::Star
            | TokenType::Slash
            | TokenType::Percent => Some(self.clone()),

            _ => None,
        }
    }

    /// Check if this type can be used with a unary operation
    /// Returns the result type if compatible, None if not
    pub fn unary_op_result(&self, op: &TokenType) -> Option<Type> {
        match op {
            TokenType::Bang => {
                // ! (not) only works on Bool operands
                if matches!(self, Type::Base(BaseType::Bool)) {
                    Some(Type::Base(BaseType::Bool))
                } else {
                    None // Error: can't use ! on non-bool types
                }
            }
            TokenType::Minus | TokenType::Plus => {
                // - and + return the same type as the operand (only valid for numeric types)
                Some(self.clone())
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub typ: Type,
    pub initializer: Option<Box<crate::ast::Expression>>,
}

#[derive(Debug)]
pub struct Scope {
    pub symbols: HashMap<String, Variable>,
    pub functions: HashMap<String, Function>,
}

impl Scope {
    pub fn new() -> Self {
        Scope {
            symbols: HashMap::new(),
            functions: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub args: Vec<Variable>,
    pub return_type: Type,
    pub body: Block,
}
