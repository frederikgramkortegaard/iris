use crate::ast::{Expression, Program, Statement};
use crate::types::{BaseType, Function, Scope, Type, Variable};
use crate::visitor::{DiagnosticCollector, Visitor};
use std::cell::RefCell;
use std::rc::Rc;

/// Visitor that Typechecks the AST structure
pub struct TypecheckingPass {
    diagnostics: DiagnosticCollector,
    scope_stack: Vec<Rc<RefCell<Scope>>>,
    current_function_return_type: Option<Type>,
}

impl TypecheckingPass {
    pub fn new() -> Self {
        TypecheckingPass {
            diagnostics: DiagnosticCollector::new(),
            scope_stack: Vec::new(),
            current_function_return_type: None,
        }
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
        let mut global_scope = Scope::new();

        // Add all global variables to the global scope
        for global in &mut program.globals {
            self.visit_variable(global);
            global_scope.symbols.insert(global.name.clone(), global.clone());
        }

        // Add all function declarations to the global scope
        for function in &program.functions {
            global_scope.functions.insert(function.name.clone(), function.clone());
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
        let mut scope = Scope::new();

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
                    variable.typ = init_type.clone();
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
            Statement::Expression(expr) => {
                self.visit_expression(expr);
            }
            Statement::Return(maybe_expr) => {
                let expr_type = match maybe_expr {
                    Some(expr) => self.visit_expression(expr)?,
                    None => Type::Base(BaseType::Void),
                };

                let Some(expected_type) = self.current_function_return_type.clone() else {
                    return None;
                };

                if !expr_type.is_equal(&expected_type) {
                    self.diagnostics_mut().error(format!(
                        "Type mismatch in return statement: expected {:?}, found {:?}",
                        expected_type, expr_type
                    ));
                }
            }
            Statement::Block(b) => {
                // Create and push scope for bare block
                let block_scope = Rc::new(RefCell::new(Scope::new()));
                b.scope = Some(Rc::clone(&block_scope));
                self.scope_stack.push(block_scope);
                self.visit_block(b);
                self.scope_stack.pop();
            }
            Statement::Assignment { left, typ, right } => {
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
                                })
                            }

                            // Concrete type without initializer - OK
                            (concrete_type, None) => self.add_variable_to_current_scope(Variable {
                                name: left.clone(),
                                typ: concrete_type.clone(),
                                initializer: None,
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
            } => {
                // Check that condition is bool
                if let Some(cond_type) = self.visit_expression(condition) {
                    if !matches!(cond_type, Type::Base(BaseType::Bool)) {
                        self.diagnostics_mut()
                            .error(format!("if condition must be bool, found {:?}", cond_type));
                    }
                }

                // Create and push scope for then block
                let then_scope = Rc::new(RefCell::new(Scope::new()));
                then.scope = Some(Rc::clone(&then_scope));
                self.scope_stack.push(then_scope);
                self.visit_block(then);
                self.scope_stack.pop();

                // Create and push scope for else block if it exists
                if let Some(else_block) = els {
                    let else_scope = Rc::new(RefCell::new(Scope::new()));
                    else_block.scope = Some(Rc::clone(&else_scope));
                    self.scope_stack.push(else_scope);
                    self.visit_block(else_block);
                    self.scope_stack.pop();
                }
            }
            Statement::While { condition, body } => {
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
                let while_scope = Rc::new(RefCell::new(Scope::new()));
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
            Expression::Variable(identifier) => {
                if let Some(var) = self.find_variable(identifier) {
                    Some(var.typ)
                } else {
                    self.diagnostics_mut()
                        .error(format!("Unknown variable: '{}'", identifier));
                    None
                }
            }
            Expression::Number(_) => Some(Type::Base(BaseType::F64)),
            Expression::UnaryOp { left, op } => {
                let operand_type = self.visit_expression(left)?;
                operand_type.unary_op_result(&op.tag)
            }
            Expression::BinaryOp { left, op, right } => {
                let left_type = self.visit_expression(left)?;
                let right_type = self.visit_expression(right)?;

                match left_type.binop_result(&op.tag, &right_type) {
                    Some(result_type) => Some(result_type),
                    None => {
                        self.diagnostics_mut().error(format!(
                            "Type mismatch in binary operation: {:?} and {:?} are not compatible",
                            left_type, right_type
                        ));
                        None
                    }
                }
            }
            Expression::Call { identifier, args } => {
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

                    Some(func.return_type.clone())
                } else {
                    self.diagnostics_mut()
                        .error(format!("Unknown function: '{}'", identifier));
                    None
                }
            }
        }
    }
}
