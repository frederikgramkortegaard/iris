//! Runs the end-to-end suite (tests/e2e.sh), which compiles every
//! examples/*.iris, executes the wasm with the wasmtime CLI, and checks
//! results against the `# EXPECTED:` annotations in each source file.
//! Skips with a message if wasmtime is not installed.

use std::process::Command;

#[test]
fn e2e_examples() {
    let wasmtime_available = Command::new("wasmtime")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !wasmtime_available {
        eprintln!("skipping e2e tests: wasmtime not found on PATH");
        return;
    }

    let status = Command::new("bash")
        .arg("tests/e2e.sh")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("failed to spawn bash");

    assert!(status.success(), "e2e suite failed, see output above");
}
