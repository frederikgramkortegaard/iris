use crate::ast::{Expression, Program, Statement};
use crate::frontend::TokenType;
use crate::hir::visitor::{DiagnosticCollector, Visitor};
use crate::mir::{
    BasicBlock, BlockId, Instruction, MirFunction, MirProgram, MirType, Opcode, Operand, Reg,
    Terminator,
};
use crate::types::{BaseType, Function, Type};
use std::collections::HashMap;

/// Pass that lowers HIR (AST) to MIR
pub struct LoweringPass {
    diagnostics: DiagnosticCollector,
    functions: Vec<MirFunction>,
    scope_stack: Vec<HashMap<String, Reg>>,
    register_cursor: usize,
    current_function: Option<MirFunction>,
    current_block: Option<BlockId>,
}

impl LoweringPass {
    pub fn new() -> Self {
        LoweringPass {
            diagnostics: DiagnosticCollector::new(),
            functions: Vec::new(),
            scope_stack: Vec::new(),
            register_cursor: 0,
            current_function: None,
            current_block: None,
        }
    }

    /// Lower the HIR program to MIR and return the MIR functions
    pub fn lower(&mut self, program: &mut Program) -> MirProgram {
        self.visit_program(program);
        MirProgram {
            functions: std::mem::take(&mut self.functions),
        }
    }

    fn push_scope(&mut self) {
        self.scope_stack.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn get_free_register(&mut self) -> Reg {
        let reg = self.register_cursor;
        self.register_cursor += 1;
        reg
    }

    /// Allocate a variable in the current scope
    fn alloc_variable(&mut self, name: String) -> Reg {
        let reg = self.get_free_register();
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.insert(name, reg);
        }
        reg
    }

    /// Lookup a variable by searching from innermost to outermost scope
    fn lookup_variable(&self, name: &str) -> Option<Reg> {
        // Search from innermost (last) to outermost (first)
        for scope in self.scope_stack.iter().rev() {
            if let Some(&reg) = scope.get(name) {
                return Some(reg);
            }
        }
        None
    }

    /// Allocate a new basic block in the current function
    fn allocate_block(&mut self) -> BlockId {
        let func = self.current_function.as_mut().expect("No current function");
        func.arena.alloc(BasicBlock {
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
            phi_nodes: Vec::new(),
        })
    }

    /// Add an instruction to the current basic block
    fn add_instruction(&mut self, inst: Instruction) {
        let block_id = self.current_block.expect("No current block");
        self.add_instruction_to_block(block_id, inst);
    }

    /// Add an instruction to a specific basic block
    fn add_instruction_to_block(&mut self, block_id: BlockId, inst: Instruction) {
        let func = self.current_function.as_mut().expect("No current function");
        func.block_mut(block_id).instructions.push(inst);
    }

    /// Set the terminator for the current basic block
    fn set_terminator(&mut self, term: Terminator) {
        let block_id = self.current_block.expect("No current block");
        self.set_terminator_for_block(block_id, term);
    }

    /// Set the terminator for a specific basic block
    fn set_terminator_for_block(&mut self, block_id: BlockId, term: Terminator) {
        let func = self.current_function.as_mut().expect("No current function");
        func.block_mut(block_id).terminator = term;
    }

    /// Convert HIR Type to MIR Type
    fn convert_type(&self, typ: &Type) -> MirType {
        match typ {
            Type::Base(base) => match base {
                BaseType::F8 => MirType::F8,
                BaseType::F16 => MirType::F16,
                BaseType::F32 => MirType::F32,
                BaseType::F64 => MirType::F64,
                BaseType::Bool => MirType::I1,
                BaseType::Void => MirType::Void, // We use this when lowering again, currently in
                // our three-address mode we require a destination
                // for any instruction, instead of making that
                // optional we'll just know that later on Void
                // types won't actually need to return anything,
                BaseType::Auto => {
                    unreachable!()
                } // We should never be here, type inference
                  // should've solved this already.
            },
            Type::PointerType(_) => {
                panic!("Not Yet Implemented")
            }
        }
    }
}

impl Visitor for LoweringPass {
    type Output = Option<Operand>;

    fn diagnostics(&self) -> &DiagnosticCollector {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut DiagnosticCollector {
        &mut self.diagnostics
    }

    fn visit_program(&mut self, program: &mut Program) -> Self::Output {
        self.push_scope();
        for glob in &mut program.globals {
            // Allocate variable in current scope which is the global one
            self.alloc_variable(glob.name.clone());
        }
        for function in &mut program.functions {
            self.visit_function(function);
        }
        self.pop_scope();

        None
    }

    fn visit_function(&mut self, function: &mut Function) -> Self::Output {
        // Push function scope for parameters
        self.push_scope();

        // Convert parameters and allocate them in the function's scope
        let mut params = Vec::new();
        for arg in &function.args {
            // Allocate parameter in current scope (gets register + adds to scope)
            let reg = self.alloc_variable(arg.name.clone());
            let mir_type = self.convert_type(&arg.typ);
            params.push((reg, mir_type));
        }

        // Convert return type
        let return_type = self.convert_type(&function.return_type);

        // Create MIR function and set as current
        let mir_func = MirFunction::new(function.name.clone(), params, return_type);
        let entry_block = mir_func.entry;
        self.current_function = Some(mir_func);
        self.current_block = Some(entry_block);

        // Lower function body statements to MIR instructions
        // Note: visit_block will handle its own scope push/pop
        // which is why we're doing it manually here (to keep vars)
        for statement in &mut function.body.statements {
            self.visit_statement(statement);
        }

        // Pop function scope
        self.pop_scope();

        // Take the function and store it
        if let Some(func) = self.current_function.take() {
            self.functions.push(func);
        }
        self.current_block = None;

        None
    }

    fn visit_statement(&mut self, statement: &mut Statement) -> Self::Output {
        match statement {
            Statement::Expression { expression, .. } => {
                self.visit_expression(expression);
            }

            Statement::While {
                condition, body, ..
            } => {
                let cond_block = self.allocate_block();
                let then_block = self.allocate_block();
                let merge_block = self.allocate_block();

                self.set_terminator(Terminator::Br { target: cond_block });
                self.current_block = Some(cond_block);
                let cond = self.visit_expression(condition).unwrap();
                self.set_terminator_for_block(
                    cond_block,
                    Terminator::BrIf {
                        cond,
                        then_bb: then_block,
                        else_bb: merge_block,
                    },
                );
                self.current_block = Some(then_block);
                self.set_terminator_for_block(then_block, Terminator::Br { target: cond_block });
                self.visit_block(body);

                // If current_block changed (nested loop), set its terminator too
                if self.current_block != Some(then_block) {
                    let block_id = self.current_block.unwrap();
                    let block = self.current_function.as_ref().unwrap().block(block_id);

                    // Only set terminator if it's still Unreachable (not a return)
                    if matches!(block.terminator, Terminator::Unreachable) {
                        self.set_terminator(Terminator::Br { target: cond_block });
                    }
                }
                self.current_block = Some(merge_block);
            }
            Statement::If {
                condition,
                then,
                els,
                ..
            } => {
                let then_block = self.allocate_block();
                let els_block = self.allocate_block();
                let merge_block = self.allocate_block();

                let cond = self.visit_expression(condition).unwrap();

                self.set_terminator(Terminator::BrIf {
                    cond,
                    then_bb: then_block,
                    else_bb: els_block,
                });

                self.set_terminator_for_block(
                    then_block,
                    Terminator::Br {
                        target: merge_block,
                    },
                );

                self.current_block = Some(then_block);
                self.visit_block(then);

                // If current_block changed (nested control flow), set its terminator too
                if self.current_block != Some(then_block) {
                    let block_id = self.current_block.unwrap();
                    let block = self.current_function.as_ref().unwrap().block(block_id);

                    // Only set terminator if it's still Unreachable (not a return)
                    if matches!(block.terminator, Terminator::Unreachable) {
                        self.set_terminator(Terminator::Br {
                            target: merge_block,
                        });
                    }
                }

                self.set_terminator_for_block(
                    els_block,
                    Terminator::Br {
                        target: merge_block,
                    },
                );

                self.current_block = Some(els_block);
                if let Some(e) = els {
                    self.visit_block(e);
                }

                // If current_block changed (nested control flow), set its terminator too
                if self.current_block != Some(els_block) {
                    let block_id = self.current_block.unwrap();
                    let block = self.current_function.as_ref().unwrap().block(block_id);

                    // Only set terminator if it's still Unreachable (not a return)
                    if matches!(block.terminator, Terminator::Unreachable) {
                        self.set_terminator(Terminator::Br {
                            target: merge_block,
                        });
                    }
                }

                self.current_block = Some(merge_block);
            }
            Statement::Block { block, .. } => {
                self.visit_block(block);
            }
            Statement::Return { expression, .. } => {
                let value = expression
                    .as_mut()
                    .and_then(|expr| self.visit_expression(expr));
                self.set_terminator(Terminator::Ret { value });
            }
            Statement::Assignment { left, right, .. } => {
                // Get destination register
                let dest_reg = self
                    .lookup_variable(left)
                    .unwrap_or_else(|| self.alloc_variable(left.clone()));

                // Lower RHS if present
                if let Some(expr) = right {
                    if let Some(value) = self.visit_expression(expr) {
                        // Get type from expression (set by typechecker)
                        let mir_type = expr.typ().as_ref().map(|t| self.convert_type(t)).unwrap();

                        self.add_instruction(Instruction {
                            dest: dest_reg,
                            op: Opcode::Copy,
                            typ: mir_type,
                            args: vec![value],
                        });
                    }
                }
            }
            _ => {}
        };

        None
    }

    fn visit_block(&mut self, block: &mut crate::ast::Block) -> Self::Output {
        self.push_scope();

        // Pre-allocate all variables from the HIR scope
        if let Some(hir_scope) = &block.scope {
            for (var_name, _var) in &hir_scope.borrow().symbols {
                self.alloc_variable(var_name.clone());
            }
        }

        // Now traverse and generate instructions
        for statement in &mut block.statements {
            self.visit_statement(statement);
        }

        self.pop_scope();
        None
    }

    fn visit_expression(&mut self, expression: &mut Expression) -> Self::Output {
        match expression {
            Expression::Number { value, .. } => {
                // Return immediate value
                Some(Operand::ImmF64(*value))
            }
            Expression::Boolean { value, .. } => {
                // Return immediate boolean
                Some(Operand::ImmBool(*value))
            }
            Expression::Variable { name, .. } => {
                // Look up variable's register
                let Some(reg) = self.lookup_variable(name) else {
                    self.diagnostics_mut()
                        .error(format!("Variable '{}' not found", name));
                    return None;
                };
                Some(Operand::Reg(reg))
            }
            Expression::BinaryOp {
                left,
                op,
                right,
                typ,
                ..
            } => {
                // Lower both operands
                let left_op = self.visit_expression(left)?;
                let right_op = self.visit_expression(right)?;

                // Allocate result register
                let result_reg = self.get_free_register();

                // Determine opcode from token
                use crate::frontend::TokenType;
                let mir_op = match op.tag {
                    TokenType::Plus => Opcode::Add,
                    TokenType::Minus => Opcode::Sub,
                    TokenType::Star => Opcode::Mul,
                    TokenType::Slash => Opcode::Div,
                    TokenType::Percent => Opcode::Mod,
                    TokenType::Equal => Opcode::Eq,
                    TokenType::NotEqual => Opcode::Ne,
                    TokenType::Less => Opcode::Lt,
                    TokenType::LessEqual => Opcode::Le,
                    TokenType::Greater => Opcode::Gt,
                    TokenType::GreaterEqual => Opcode::Ge,
                    _ => {
                        self.diagnostics_mut()
                            .error(format!("Unsupported binary operator: {:?}", op.tag));
                        return None;
                    }
                };

                // Add instruction
                let mir_type = typ.as_ref().map(|t| self.convert_type(t)).unwrap();
                self.add_instruction(Instruction {
                    dest: result_reg,
                    op: mir_op,
                    typ: mir_type,
                    args: vec![left_op, right_op],
                });

                Some(Operand::Reg(result_reg))
            }
            Expression::UnaryOp { left, op, .. } => {
                match op.tag {
                    TokenType::Minus => {
                        let val = self.visit_expression(left).unwrap();
                        let dest = self.get_free_register();
                        let mir_type = left.typ().as_ref().map(|t| self.convert_type(t)).unwrap();
                        self.add_instruction(Instruction {
                            dest,
                            op: Opcode::Sub,
                            typ: mir_type,
                            args: vec![Operand::ImmF64(0.0), val],
                        });
                        return Some(Operand::Reg(dest));
                    }
                    TokenType::Bang => {
                        let val = self.visit_expression(left).unwrap();
                        let dest = self.get_free_register();
                        self.add_instruction(Instruction {
                            dest,
                            op: Opcode::Eq,
                            typ: MirType::I1,
                            args: vec![Operand::ImmF64(0.0), val],
                        });
                        return Some(Operand::Reg(dest));
                    }
                    _ => {}
                }
                self.diagnostics_mut()
                    .error("Unary operations not yet implemented".to_string());
                None
            }
            Expression::Call {
                identifier,
                args,
                typ,
                ..
            } => {
                let dest = self.get_free_register();
                let mut operands: Vec<Operand> = Vec::new();
                operands.push(Operand::Label(identifier.clone()));
                for arg in args {
                    operands.push(self.visit_expression(arg).unwrap());
                }
                self.add_instruction(Instruction {
                    dest,
                    op: Opcode::Call,
                    typ: typ.as_ref().map(|t| self.convert_type(t)).unwrap(),
                    args: operands,
                });
                Some(Operand::Reg(dest))
            }
        }
    }
}
