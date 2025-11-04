use iris::ast::Program;
use iris::visitor::Visitor;

pub struct PassManager {
    passes: Vec<Box<dyn Visitor>>,
}

impl PassManager {
    pub fn run(&mut self, program: &Program) -> Result {
        let mut all_warnings = Vec::new();
        let mut all_info = Vec::new();
        let mut all_debug = Vec::new();
        for pass in &mut self.passes {
            pass.visit_program(program);

            let diagnostics = pass.diagnostics();

            for error in &diagnostics.errors() {
                eprintln!("Error: {}", error);
            }

            for warning in &diagnostics.warnings() {
                eprintln!("Warning: {}", warning);
            }
            for info in &diagnostics.info() {
                println!("info: {}", info);
            }
            for debug in &diagnostics.debug() {
                println!("Debug: {}", debug);
            }
            if diagnostics.has_errors() {
                return Err(());
            }
        }

        Ok()
    }
}
