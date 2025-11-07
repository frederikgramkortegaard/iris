//! The Iris compiler library.
//!
//! This crate provides the core functionality for the Iris compiler,
//! including lexical analysis, parsing, and code generation.

pub mod span;
pub mod frontend;
pub mod ast;
pub mod types;
pub mod diagnostics;
pub mod cli;
pub mod hir;
pub mod mir;
