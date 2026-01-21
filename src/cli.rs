use crate::frontend::{LexerContext, ParserContext};
use crate::hir::passes::ast_simplification::ASTSimplificationPass;
use crate::hir::passes::counting::CountingPass;
use crate::hir::passes::lowering::LoweringPass;
use crate::hir::passes::print::PrintPass;
use crate::hir::passes::typechecking::TypecheckingPass;
use crate::mir::passes::const_prop::MirConstPropPass;
use crate::mir::passes::dce::MirDCEPass;
use crate::mir::passes::print::MirPrintingPass;
use crate::mir::passes::ssa::MirSSAPass;
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
    let mut program = parser
        .parse()
        .map_err(|e| format!("Parse error: {}", e.message))?;

    // Run HIR passes
    program
        .run_pass(&mut CountingPass::new())?
        .run_pass(&mut PrintPass::new())?
        .run_pass(&mut ASTSimplificationPass::new())?
        .run_pass(&mut TypecheckingPass::new())?;

    // Lower HIR to MIR
    let mut lowering_pass = LoweringPass::new();
    let mut mir = lowering_pass.lower(&mut program);

    // Run MIR passes
    mir.run_pass(&mut MirSSAPass::new())?
        .run_pass(&mut MirConstPropPass::new())?
        .run_pass(&mut MirDCEPass::new())?
        .run_pass(&mut MirPrintingPass::new())?;

    Ok(())
}
