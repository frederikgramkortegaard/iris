use std::collections::HashMap;
use crate::ast::Block;

#[derive(Debug)]
pub enum BaseType {
    F8,
    F16,
    F32,
    F64,
    Void,
    Auto,
}

#[derive(Debug)]
pub enum Type {
    Base(BaseType),
    PointerType(Box<Type>),
}

#[derive(Debug)]
pub struct Variable {
    pub name: String,
    pub typ: Type,
    pub initializer: Option<Box<crate::ast::Expression>>,
}

#[derive(Debug)]
pub struct Scope {
    // Note: Parent scope relationships should be tracked separately
    // during semantic analysis (e.g., in a HashMap<ScopeId, ScopeId>)
    // rather than storing references here to avoid lifetime complexity.
    pub symbols: HashMap<String, Variable>,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub args: Vec<Variable>,
    pub return_type: Type,
    pub body: Block,
}
