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

use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_verbose(v: bool) {
    VERBOSE.store(v, Ordering::Relaxed);
}

pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}
