use crate::lexer::LexerContext;
use crate::parser::ParserContext;
use crate::passes::counting::CountingPass;
use crate::passes::ast_const_folding::ASTConstFoldingPass;
use crate::passes::print::PrintPass;
use crate::passes::typechecking::TypecheckingPass;
use crate::visitor::Visitor;
use std::fs;

/// Runs the compiler CLI with the given command-line arguments.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <input-file>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];

    // Read the input file
    let input = fs::read_to_string(filename)
        .map_err(|e| format!("Failed to read file '{}': {}", filename, e))?;

    // Lex the input
    let tokens = LexerContext::lex(&input).map_err(|e| {
        format!(
            "Lexing error at line {}, column {}: {}",
            e.row, e.column, e.message
        )
    })?;

    // Parse the tokens
    let mut parser = ParserContext::new(tokens);
    let mut program = parser.parse().map_err(|e| {
        format!("Parse error: {}", e.message)
    })?;

    // Run counting pass
    let mut counting_pass = CountingPass::new();
    counting_pass.visit_program(&mut program);
    print_diagnostics(&counting_pass);
    if counting_pass.diagnostics().has_errors() {
        return Err("Compilation failed due to errors".into());
    }

    // Run print pass
    let mut print_pass = PrintPass::new();
    print_pass.visit_program(&mut program);
    print_diagnostics(&print_pass);
    if print_pass.diagnostics().has_errors() {
        return Err("Compilation failed due to errors".into());
    }

    // Run typechecking pass
    let mut typechecking_pass = TypecheckingPass::new();
    typechecking_pass.visit_program(&mut program);
    print_diagnostics(&typechecking_pass);
    if typechecking_pass.diagnostics().has_errors() {
        return Err("Compilation failed due to errors".into());
    }
    //
    // Run AST-level constant folding pass
    let mut ast_const_folding_pass = ASTConstFoldingPass::new();

    ast_const_folding_pass.visit_program(&mut program);
    print_diagnostics(&ast_const_folding_pass);
    if ast_const_folding_pass.diagnostics().has_errors() {
        return Err("Compilation failed due to errors".into());
    }

    Ok(())
}

/// Helper function to print diagnostics from a visitor
fn print_diagnostics<V: Visitor>(visitor: &V) {
    let diagnostics = visitor.diagnostics();

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
}
