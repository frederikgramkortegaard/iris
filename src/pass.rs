use crate::diagnostics::DiagnosticCollector;
use log::debug;

/// Generic trait for compiler passes that can be run in a pipeline.
pub trait Pass<IR> {
    fn run(&mut self, ir: &mut IR);
    fn diagnostics(&self) -> &DiagnosticCollector;
}

/// Extension trait to add pass running methods to any IR type.
pub trait RunPass {
    fn run_pass<P: Pass<Self>>(
        &mut self,
        pass: &mut P,
    ) -> Result<&mut Self, Box<dyn std::error::Error>>
    where
        Self: Sized;
}

impl<IR> RunPass for IR {
    fn run_pass<P: Pass<IR>>(
        &mut self,
        pass: &mut P,
    ) -> Result<&mut Self, Box<dyn std::error::Error>> {
        pass.run(self);
        print_diagnostics(pass.diagnostics())?;
        Ok(self)
    }
}

/// Print collected diagnostics and fail if any of them are errors.
pub(crate) fn print_diagnostics(
    diagnostics: &DiagnosticCollector,
) -> Result<(), Box<dyn std::error::Error>> {
    for error in &diagnostics.errors {
        eprintln!("Error: {}", error);
    }
    for warning in &diagnostics.warnings {
        eprintln!("Warning: {}", warning);
    }
    for info in &diagnostics.info {
        debug!("Info: {}", info);
    }

    if diagnostics.has_errors() {
        return Err("Compilation failed due to errors".into());
    }

    Ok(())
}
