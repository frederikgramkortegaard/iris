use crate::codegen::wasm::{lower, peephole};
use crate::codegen::Emit;
use crate::frontend::{LexerContext, ParserContext};
use crate::hir::passes::counting::CountingPass;
use crate::hir::passes::lowering::LoweringPass;
use crate::hir::passes::print::PrintPass;
use crate::hir::passes::simplify::SimplifyPass;
use crate::hir::passes::typechecking::TypecheckingPass;
use crate::mir::passes::const_prop::MirConstPropPass;
use crate::mir::passes::copy_prop::MirCopyPropPass;
use crate::mir::passes::dbe::MirDeadBlockEliminationPass;
use crate::mir::passes::dce::MirDCEPass;
use crate::mir::passes::deconstruct::MirSSADeconstructionPass;
use crate::mir::passes::gvn::MirGVNPass;
use crate::mir::passes::loops::MirLoopPass;
use crate::mir::passes::print::MirPrintingPass;
use crate::mir::passes::reg_compact::RegCompactPass;
use crate::mir::passes::scev::MirSCEVPass;
use crate::mir::passes::ssa::MirSSAPass;
use crate::mir::passes::tailcall::MirTailCallPass;

use crate::pass::RunPass;

use std::fs;

/// Runs the compiler CLI with the given command-line arguments.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} <input-file> [-o <output-file>] [-t <target>] [--verbose]",
            args[0]
        );
        std::process::exit(1);
    }

    let filename = &args[1];

    // Parse flags
    let verbose = args.iter().any(|a| a == "--verbose");
    crate::set_verbose(verbose);

    let output_path = args.iter().position(|a| a == "-o").map(|i| {
        args.get(i + 1)
            .unwrap_or_else(|| {
                eprintln!("Error: -o requires an output file path");
                std::process::exit(1);
            })
            .clone()
    });

    let target = args
        .iter()
        .position(|a| a == "-t")
        .map(|i| {
            args.get(i + 1)
                .unwrap_or_else(|| {
                    eprintln!("Error: -t requires a target (e.g. wasm)");
                    std::process::exit(1);
                })
                .clone()
        })
        .unwrap_or_else(|| "wasm".to_string());

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
        .run_pass(&mut TypecheckingPass::new())?
        .run_pass(&mut SimplifyPass::new())?;

    if verbose {
        program.run_pass(&mut PrintPass::with_message("After HIR"))?;
    }

    // Lower HIR to MIR
    let mut lowering_pass = LoweringPass::new();
    let mut mir = lowering_pass.lower(&mut program);

    // Run MIR passes
    mir.run_pass(&mut MirTailCallPass::new())?
        .run_pass(&mut MirSSAPass::new())?;

    // Optimize in SSA form
    for _ in 0..3 {
        mir.run_pass(&mut MirConstPropPass::new())?
            .run_pass(&mut MirLoopPass::new())?
            .run_pass(&mut MirGVNPass::new())?
            .run_pass(&mut MirCopyPropPass::new())?
            .run_pass(&mut MirDCEPass::new())?;
    }

    if verbose {
        mir.run_pass(&mut MirPrintingPass::with_message("post optim"))?;
    }
    mir.run_pass(&mut MirSCEVPass::new())?;

    if verbose {
        mir.run_pass(&mut MirPrintingPass::with_message("Optimized SSA"))?;
    }

    mir.run_pass(&mut MirSSADeconstructionPass::new())?
        .run_pass(&mut RegCompactPass::new())?
        .run_pass(&mut MirDeadBlockEliminationPass::new())?;

    let output = match target.as_str() {
        "wasm" => {
            let mut wat_module = lower::lower(&mir);
            peephole::peephole(&mut wat_module);
            wat_module.emit()
        }
        _ => return Err(format!("Unknown target: {}", target).into()),
    };

    if let Some(path) = &output_path {
        fs::write(path, &output)
            .map_err(|e| format!("Failed to write output file '{}': {}", path, e))?;
    } else {
        println!("{}", output);
    }

    Ok(())
}
