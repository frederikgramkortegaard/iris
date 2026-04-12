use crate::ast::{Expression, Program, Statement};
use crate::hir::passes::HirPass;
use crate::hir::visitor::{DiagnosticCollector, Visitor};
use crate::types::{BaseType, Function, Scope, Type, Variable};
use std::cell::RefCell;
use std::rc::Rc;

/// Visitor that Typechecks the AST structure
pub struct TypecheckingPass {
    diagnostics: DiagnosticCollector,
    scope_stack: Vec<Rc<RefCell<Scope>>>,
    current_function_return_type: Option<Type>,
    next_scope_id: usize,
}

impl Default for TypecheckingPass {
    fn default() -> Self {
        Self::new()
    }
}

impl TypecheckingPass {
    pub fn new() -> Self {
        TypecheckingPass {
            diagnostics: DiagnosticCollector::new(),
            scope_stack: Vec::new(),
            current_function_return_type: None,
            next_scope_id: 0,
        }
    }

    fn allocate_scope_id(&mut self) -> usize {
        let id = self.next_scope_id;
        self.next_scope_id += 1;
        id
    }

    pub fn find_variable(&self, name: &str) -> Option<Variable> {
        self.scope_stack
            .iter()
            .rev()
            .find_map(|scope| scope.borrow().symbols.get(name).cloned())
    }

    pub fn find_variable_in_current_scope(&self, name: &str) -> Option<Variable> {
        self.scope_stack
            .last()
            .and_then(|scope| scope.borrow().symbols.get(name).cloned())
    }

    pub fn add_variable_to_current_scope(&mut self, var: Variable) {
        if let Some(scope_rc) = self.scope_stack.last() {
            scope_rc.borrow_mut().symbols.insert(var.name.clone(), var);
        }
    }

    pub fn find_function(&self, name: &str) -> Option<Function> {
        self.scope_stack
            .iter()
            .rev()
            .find_map(|scope| scope.borrow().functions.get(name).cloned())
    }
}

impl Visitor for TypecheckingPass {
    type Output = Option<Type>;

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_program(&mut self, program: &mut Program) -> Self::Output {
        // Create a global scope for globals and function declarations
        let mut global_scope = Scope::new(self.allocate_scope_id());

        // Add all global variables to the global scope
        for global in &mut program.globals {
            self.visit_variable(global);
            global_scope
                .symbols
                .insert(global.name.clone(), global.clone());
        }

        // Add all function declarations to the global scope
        for function in &program.functions {
            global_scope
                .functions
                .insert(function.name.clone(), function.clone());
        }

        // Push global scope to stack
        let global_scope_rc = Rc::new(RefCell::new(global_scope));
        self.scope_stack.push(global_scope_rc);

        // Visit all functions
        for function in &mut program.functions {
            self.visit_function(function);
        }

        // Pop global scope
        self.scope_stack.pop();

        None
    }

    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        // Create a scope for the function's body
        let mut scope = Scope::new(self.allocate_scope_id());

        // Add the function parameters to the scope
        for arg in &mut function.args {
            self.visit_variable(arg);

            scope.symbols.insert(arg.name.clone(), arg.clone());
        }

        scope
            .functions
            .insert(function.name.clone(), function.clone());

        let scope_rc = Rc::new(RefCell::new(scope));
        function.body.scope = Some(Rc::clone(&scope_rc));
        self.scope_stack.push(scope_rc);

        // Set the current function's return type so Return statements can check against it
        self.current_function_return_type = Some(function.return_type.clone());

        for statement in &mut function.body.statements {
            self.visit_statement(statement);
        }

        // Clear the current function return type
        self.current_function_return_type = None;

        // Pop the function scope
        self.scope_stack.pop();

        None
    }

    fn visit_variable(&mut self, variable: &mut Variable) -> Self::Output {
        match (&variable.typ, &mut variable.initializer) {
            // Auto with no initializer is an error
            (Type::Base(BaseType::Auto), None) => {
                self.diagnostics_mut().error(format!(
                    "Variable '{}' has type 'auto' but no initializer to infer type from",
                    variable.name
                ));
                return None;
            }
            // Auto with initializer - infer the type
            (Type::Base(BaseType::Auto), Some(init)) => {
                if let Some(init_type) = self.visit_expression(init) {
                    variable.typ = init_type;
                } else {
                    return None;
                }
            }
            // Concrete type with no initializer - that's fine
            (_, None) => {
                // no-op
            }
            // Concrete type with initializer - check they match
            (_, Some(init)) => {
                if let Some(init_type) = self.visit_expression(init) {
                    if !variable.typ.is_equal(&init_type) {
                        self.diagnostics_mut().error(format!(
                            "Type mismatch for variable '{}': expected {:?}, found {:?}",
                            variable.name, variable.typ, init_type
                        ));
                    }
                }
            }
        }

        Some(variable.typ.clone())
    }

    fn visit_statement(&mut self, statement: &mut Statement) -> Self::Output {
        match statement {
            Statement::Expression { expression, .. } => {
                self.visit_expression(expression);
            }
            Statement::Return {
                expression: maybe_expr,
                ..
            } => {
                let expr_type = match maybe_expr {
                    Some(expr) => self.visit_expression(expr)?,
                    None => Type::Base(BaseType::Void),
                };

                let expected_type = self.current_function_return_type.clone()?;

                if !expr_type.is_equal(&expected_type) {
                    self.diagnostics_mut().error(format!(
                        "Type mismatch in return statement: expected {:?}, found {:?}",
                        expected_type, expr_type
                    ));
                }
            }
            Statement::Block { block: b, .. } => {
                // Create and push scope for bare block
                let block_scope = Rc::new(RefCell::new(Scope::new(self.allocate_scope_id())));
                b.scope = Some(Rc::clone(&block_scope));
                self.scope_stack.push(block_scope);
                self.visit_block(b);
                self.scope_stack.pop();
            }
            Statement::Assignment {
                left, typ, right, ..
            } => {
                match typ.as_ref() {
                    // Declaration: check current scope only for redeclaration
                    Some(t) => {
                        if self.find_variable_in_current_scope(left).is_some() {
                            self.diagnostics_mut().error(format!(
                                "Redeclaration of variable in same scope: {:?}",
                                left
                            ));
                            return None;
                        }

                        // Handle type checking based on type and initializer
                        match (t, right.as_mut()) {
                            // Auto with initializer - infer type
                            (Type::Base(BaseType::Auto), Some(r)) => {
                                let right_type = self.visit_expression(r)?;
                                self.add_variable_to_current_scope(Variable {
                                    name: left.clone(),
                                    typ: right_type,
                                    read_only: false,
                                    initializer: right.clone(),
                                })
                            }

                            // Auto without initializer - error
                            (Type::Base(BaseType::Auto), None) => {
                                self.diagnostics_mut().error(format!(
                                    "Variable '{}' has type 'auto' but no initializer to infer type from",
                                    left
                                ));
                                return None;
                            }

                            // Concrete type with initializer - check match
                            (concrete_type, Some(r)) => {
                                let right_type = self.visit_expression(r)?;
                                if !concrete_type.is_equal(&right_type) {
                                    self.diagnostics_mut().error(format!(
                                        "Type mismatch for variable '{}': expected {:?}, found {:?}",
                                        left, concrete_type, right_type
                                    ));
                                }

                                self.add_variable_to_current_scope(Variable {
                                    name: left.clone(),
                                    typ: concrete_type.clone(),
                                    initializer: right.clone(),
                                    read_only: false,
                                })
                            }

                            // Concrete type without initializer - OK
                            (concrete_type, None) => self.add_variable_to_current_scope(Variable {
                                name: left.clone(),
                                typ: concrete_type.clone(),
                                initializer: None,
                                read_only: false,
                            }),
                        }
                    }

                    // Reassignment: check all scopes
                    None => {
                        let Some(var) = self.find_variable(left) else {
                            self.diagnostics_mut()
                                .error(format!("Cannot assign to undeclared variable '{}'", left));
                            return None;
                        };

                        if var.read_only {
                            self.diagnostics_mut()
                                .error(format!("Cannot assign to a read-only variable '{}' note, this can be e.g. the identifier specified in a for-loop", left));
                            return None;
                        }

                        if let Some(r) = right.as_mut() {
                            let right_type = self.visit_expression(r)?;
                            if !var.typ.is_equal(&right_type) {
                                self.diagnostics_mut().error(format!(
                                    "Type mismatch in assignment to '{}': expected {:?}, found {:?}",
                                    left, var.typ, right_type
                                ));
                            }
                        } else {
                            unreachable!("Parser should not produce reassignment with no value");
                        }
                    }
                }
            }
            Statement::If {
                condition,
                then,
                els,
                ..
            } => {
                // Check that condition is bool
                if let Some(cond_type) = self.visit_expression(condition) {
                    if !matches!(cond_type, Type::Base(BaseType::Bool)) {
                        self.diagnostics_mut()
                            .error(format!("if condition must be bool, found {:?}", cond_type));
                    }
                }

                // Create and push scope for then block
                let then_scope = Rc::new(RefCell::new(Scope::new(self.allocate_scope_id())));
                then.scope = Some(Rc::clone(&then_scope));
                self.scope_stack.push(then_scope);
                self.visit_block(then);
                self.scope_stack.pop();

                // Create and push scope for else block if it exists
                if let Some(else_block) = els {
                    let else_scope = Rc::new(RefCell::new(Scope::new(self.allocate_scope_id())));
                    else_block.scope = Some(Rc::clone(&else_scope));
                    self.scope_stack.push(else_scope);
                    self.visit_block(else_block);
                    self.scope_stack.pop();
                }
            }

            Statement::For {
                ident, range, body, ..
            } => {
                // @TODO : this could probably done in parsing instead of typechecking
                if !matches!(range.as_ref(), Expression::Range { .. }) {
                    self.diagnostics_mut()
                        .error("For loop requires a range expression".to_string());
                    return None;
                }
                let range_type = self.visit_expression(range)?;

                // Create and push scope for while body
                let for_scope = Rc::new(RefCell::new(Scope::new(self.allocate_scope_id())));
                body.scope = Some(Rc::clone(&for_scope));
                self.scope_stack.push(for_scope);

                // If we have an identifier we assign to the range, e.g. "for ID in range" then we
                // add ID to the new scope we made in the for loop, it will have the same type as
                // the range elements have
                if let Some(id) = ident {
                    let elem_type = range_type.clone();

                    self.add_variable_to_current_scope(Variable {
                        name: id.clone(),
                        typ: elem_type,
                        initializer: None,
                        read_only: true,
                    })
                }
                self.visit_block(body);
                self.scope_stack.pop();
            }
            Statement::While {
                condition, body, ..
            } => {
                // Check that condition is bool
                if let Some(cond_type) = self.visit_expression(condition) {
                    if !matches!(cond_type, Type::Base(BaseType::Bool)) {
                        self.diagnostics_mut().error(format!(
                            "While condition must be bool, found {:?}",
                            cond_type
                        ));
                    }
                }

                // Create and push scope for while body
                let while_scope = Rc::new(RefCell::new(Scope::new(self.allocate_scope_id())));
                body.scope = Some(Rc::clone(&while_scope));
                self.scope_stack.push(while_scope);
                self.visit_block(body);
                self.scope_stack.pop();
            }
            _ => {
                self.diagnostics_mut()
                    .error(format!("Unhandled statement type: {:?}", statement));
            }
        }
        None
    }

    fn visit_expression(&mut self, expression: &mut Expression) -> Self::Output {
        match expression {
            Expression::Range {
                start, end, typ, ..
            } => {
                let start_type = self.visit_expression(start)?;
                let end_type = self.visit_expression(end)?;
                if !start_type.is_equal(&end_type) {
                    self.diagnostics_mut().error(
                        "Both start and end need to be the same type in a range".to_string(),
                    );
                    None
                } else {
                    *typ = Some(start_type.clone());
                    Some(start_type)
                }
            }
            Expression::Variable {
                name: identifier,
                typ,
                ..
            } => {
                if let Some(var) = self.find_variable(identifier) {
                    *typ = Some(var.typ.clone());
                    Some(var.typ)
                } else {
                    self.diagnostics_mut()
                        .error(format!("Unknown variable: '{}'", identifier));
                    None
                }
            }
            Expression::Number { typ, .. } => {
                let t = Type::Base(BaseType::F64);
                *typ = Some(t.clone());
                Some(t)
            }
            Expression::Boolean { typ, .. } => {
                let t = Type::Base(BaseType::Bool);
                *typ = Some(t.clone());
                Some(t)
            }
            Expression::UnaryOp { left, op, typ, .. } => {
                let operand_type = self.visit_expression(left)?;
                match operand_type.unary_op_result(&op.tag) {
                    Some(result_type) => {
                        *typ = Some(result_type.clone());
                        Some(result_type)
                    }
                    None => {
                        self.diagnostics_mut().error(format!(
                            "Invalid unary operation: operator '{}' cannot be applied to type {:?}",
                            op.lexeme, operand_type
                        ));
                        None
                    }
                }
            }
            Expression::BinaryOp {
                left,
                op,
                right,
                typ,
                ..
            } => {
                let left_type = self.visit_expression(left)?;
                let right_type = self.visit_expression(right)?;

                match left_type.binop_result(&op.tag, &right_type) {
                    Some(result_type) => {
                        *typ = Some(result_type.clone());
                        Some(result_type)
                    }
                    None => {
                        self.diagnostics_mut().error(format!(
                            "Type mismatch in binary operation: {:?} and {:?} are not compatible",
                            left_type, right_type
                        ));
                        None
                    }
                }
            }
            Expression::Call {
                identifier,
                args,
                typ,
                ..
            } => {
                if let Some(func) = &mut self.find_function(identifier) {
                    // Check argument count
                    if func.args.len() != args.len() {
                        self.diagnostics_mut().error(format!(
                            "Function '{}' expects {} arguments, got {}",
                            identifier,
                            func.args.len(),
                            args.len()
                        ));
                        return None;
                    }

                    // collect all argument types
                    let mut arg_types = Vec::new();
                    for arg_expr in args {
                        match self.visit_expression(arg_expr) {
                            Some(t) => arg_types.push(t),
                            None => return None, // Error already reported
                        }
                    }

                    // check types
                    for (param, arg_type) in func.args.iter().zip(arg_types.iter()) {
                        if !param.typ.is_equal(arg_type) {
                            self.diagnostics_mut().error(format!(
                                "Argument type mismatch for parameter '{}': expected {:?}, found {:?}",
                                param.name, param.typ, arg_type
                            ));
                        }
                    }

                    let return_type = func.return_type.clone();
                    *typ = Some(return_type.clone());
                    Some(return_type)
                } else {
                    self.diagnostics_mut()
                        .error(format!("Unknown function: '{}'", identifier));
                    None
                }
            }
        }
    }
}

impl HirPass for TypecheckingPass {
    fn run(&mut self, program: &mut Program) {
        self.visit_program(program);
    }

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::{LexerContext, ParserContext};
    use crate::hir::passes::HirPass;

    fn typecheck(input: &str) -> TypecheckingPass {
        let tokens = LexerContext::lex(input).expect("lexing failed");
        let mut program = ParserContext::new(tokens).parse().expect("parsing failed");
        let mut pass = TypecheckingPass::new();
        pass.run(&mut program);
        pass
    }

    fn has_error_containing(pass: &TypecheckingPass, substring: &str) -> bool {
        HirPass::diagnostics(pass)
            .errors
            .iter()
            .any(|e: &String| e.contains(substring))
    }

    #[test]
    fn for_loop_identifier_is_read_only() {
        let pass = typecheck("fn main() { for i in 0..10 { i = 5 } }");
        assert!(has_error_containing(&pass, "read-only"));
    }

    #[test]
    fn for_loop_valid_usage() {
        let pass = typecheck("fn main() -> f64 { for i in 0..10 { return i } return 0 }");
        assert!(HirPass::diagnostics(&pass).errors.is_empty());
    }
}
