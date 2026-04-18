//! The Iris compiler library.

pub mod ast;
pub mod cli;
pub mod codegen;
pub mod diagnostics;
pub mod frontend;
pub mod hir;
pub mod mir;
pub mod pass;
pub mod span;
pub mod types;

pub use log;
