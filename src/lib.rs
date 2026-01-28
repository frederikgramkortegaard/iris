//! The Iris compiler library.
//!
//! This crate provides the core functionality for the Iris compiler,
//! including lexical analysis, parsing, and code generation.

pub mod ast;
pub mod cli;
pub mod diagnostics;
pub mod frontend;
pub mod hir;
pub mod lir;
pub mod mir;
pub mod pass;
pub mod span;
pub mod types;
