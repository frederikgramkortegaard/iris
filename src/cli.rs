use crate::frontend::{LexerContext, ParserContext};
use crate::hir::passes::ast_simplification::ASTSimplificationPass;
use crate::hir::passes::counting::CountingPass;
use crate::hir::passes::lowering::LoweringPass;
use crate::hir::passes::print::PrintPass;
use crate::hir::passes::typechecking::TypecheckingPass;
use crate::hir::visitor::Visitor;
use crate::mir::passes::print::MirPrintingPass;
use crate::mir::passes::ssa::MirSSAPass;
use crate::mir::visitor::MirVisitor;
use std::fs;

/// Helper function to print diagnostics from a HIR visitor
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

/// Helper function to print diagnostics from a MIR visitor
fn print_mir_diagnostics<V: MirVisitor>(visitor: &V) {
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
    let mut program = parser
        .parse()
        .map_err(|e| format!("Parse error: {}", e.message))?;

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

    // Run AST simplification pass (constant folding, boolean folding, etc.)
    let mut ast_simplification_pass = ASTSimplificationPass::new();
    ast_simplification_pass.visit_program(&mut program);
    print_diagnostics(&ast_simplification_pass);
    if ast_simplification_pass.diagnostics().has_errors() {
        return Err("Compilation failed due to errors".into());
    }
    // Run typechecking pass
    let mut typechecking_pass = TypecheckingPass::new();
    typechecking_pass.visit_program(&mut program);
    print_diagnostics(&typechecking_pass);
    if typechecking_pass.diagnostics().has_errors() {
        return Err("Compilation failed due to errors".into());
    }

    // Lower HIR to MIR
    let mut lowering_pass = LoweringPass::new();
    let mut mir = lowering_pass.lower(&mut program);
    print_diagnostics(&lowering_pass);
    if lowering_pass.diagnostics().has_errors() {
        return Err("Compilation failed due to errors".into());
    }

    // Convert MIR to SSA
    let mut ssa_pass = MirSSAPass::new();
    ssa_pass.convert(&mut mir);
    print_mir_diagnostics(&ssa_pass);
    if ssa_pass.diagnostics().has_errors() {
        return Err("Compilation failed due to errors".into());
    }

   let mut mir_print_pass = MirPrintingPass::new();
   mir_print_pass.visit_program(&mut mir);
   print_mir_diagnostics(&mir_print_pass);

   println!("\nMIR: Generated {} functions", mir.functions.len());
   for func in &mir.functions {
       println!("  Function: {} ({} blocks)", func.name, func.arena.len());
   }


    Ok(())
}
