/// Collects diagnostic messages during compilation
#[derive(Default, Debug)]
pub struct DiagnosticCollector {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
    pub debug: Vec<String>,
}

impl DiagnosticCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn error(&mut self, msg: String) {
        self.errors.push(msg);
    }

    pub fn warn(&mut self, msg: String) {
        self.warnings.push(msg);
    }

    pub fn info(&mut self, msg: String) {
        self.info.push(msg);
    }

    pub fn debug(&mut self, msg: String) {
        self.debug.push(msg);
    }

    pub fn clear(&mut self) {
        self.errors.clear();
        self.warnings.clear();
        self.info.clear();
        self.debug.clear();
    }
}
