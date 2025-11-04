use crate::lexer::LexerContext;
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
    let lexer = LexerContext::new();
    let tokens = lexer.lex(&input).map_err(|e| {
        format!(
            "Lexing error at line {}, column {}: {}",
            e.row, e.column, e.message
        )
    })?;

    // Print all tokens
    for token in tokens.iter() {
        println!("{:?}", token);
    }

    Ok(())
}
