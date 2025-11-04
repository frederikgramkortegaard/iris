use crate::ast::Program;
use crate::visitor::Visitor;

pub mod counting;

pub struct PassManager {
    passes: Vec<Box<dyn Visitor>>,
}

impl PassManager {
    pub fn new() -> Self {
        PassManager {
            passes: Vec::new(),
        }
    }

    pub fn add_pass(&mut self, pass: Box<dyn Visitor>) {
        self.passes.push(pass);
    }

    pub fn run(&mut self, program: &Program) -> Result<(), ()> {
        for pass in &mut self.passes {
            pass.visit_program(program);

            let diagnostics = pass.diagnostics();

            // Print errors
            for error in &diagnostics.errors {
                eprintln!("Error: {}", error);
            }

            // Print warnings
            for warning in &diagnostics.warnings {
                eprintln!("Warning: {}", warning);
            }

            // Print info
            for info in &diagnostics.info {
                println!("Info: {}", info);
            }

            // Stop if errors
            if diagnostics.has_errors() {
                return Err(());
            }
        }

        Ok(())
    }
}
