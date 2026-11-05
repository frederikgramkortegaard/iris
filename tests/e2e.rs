//! End-to-end tests: compile every examples/*.iris with the real binary,
//! run the wasm with the wasmtime CLI and check results against the
//! `# EXPECTED:` annotations in the source file itself:
//!
//!   # EXPECTED: factorial(5) == 120
//!   # EXPECTED: simple() traps
//!
//! Arguments and results are f64. `traps` means the call must trap (e.g. a
//! non-void function that falls off the end hits unreachable).
//!
//! On top of the annotated cases, every example is compiled twice and the
//! two WAT outputs must be identical (the compiler must be deterministic),
//! and every exported function must have at least one EXPECTED line.
//! The compiler run is killed after 20s so a structurer bug can't hang the
//! suite; the wasm itself runs under wasmtime's own -W timeout.
//!
//! Skips with a message if wasmtime is not installed.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const COMPILE_TIMEOUT: Duration = Duration::from_secs(20);

struct Expectation {
    func: String,
    args: Vec<String>,
    // None means the call must trap
    want: Option<f64>,
}

/// Parse `name(arg, ...) == value` or `name(arg, ...) traps`.
fn parse_expectation(line: &str) -> Option<Expectation> {
    let (func, rest) = line.split_once('(')?;
    let func = func.trim();
    if func.is_empty() || !func.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    let (argstr, rest) = rest.split_once(')')?;
    let args: Vec<String> = argstr
        .split(',')
        .map(|a| a.trim().to_string())
        .filter(|a| !a.is_empty())
        .collect();
    let rest = rest.trim();
    if rest == "traps" {
        return Some(Expectation {
            func: func.to_string(),
            args,
            want: None,
        });
    }
    let value = rest.strip_prefix("==")?.trim();
    Some(Expectation {
        func: func.to_string(),
        args,
        want: Some(value.parse().ok()?),
    })
}

/// Run the compiler, killing it if it exceeds COMPILE_TIMEOUT.
/// Ok(stderr_first_line_on_failure): None means success.
fn compile(iris: &str, src: &Path, out: &Path) -> Result<(), String> {
    let mut child = Command::new(iris)
        .arg(src)
        .arg("-o")
        .arg(out)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn compiler");

    let start = Instant::now();
    loop {
        match child.try_wait().expect("wait on compiler") {
            Some(status) => {
                if status.success() {
                    return Ok(());
                }
                let mut err = String::new();
                if let Some(mut stderr) = child.stderr.take() {
                    use std::io::Read;
                    let _ = stderr.read_to_string(&mut err);
                }
                let first = err.lines().next().unwrap_or("no stderr");
                return Err(format!("compiler failed: {}", first));
            }
            None if start.elapsed() > COMPILE_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("compiler timed out after {:?}", COMPILE_TIMEOUT));
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

fn wat_exports(wat: &str) -> Vec<String> {
    let mut exports = Vec::new();
    for part in wat.split("(export \"").skip(1) {
        if let Some(name) = part.split('"').next() {
            exports.push(name.to_string());
        }
    }
    exports
}

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

    let iris = env!("CARGO_BIN_EXE_iris");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tmp = std::env::temp_dir().join(format!("iris-e2e-{}", std::process::id()));
    fs::create_dir_all(&tmp).expect("create temp dir");

    let mut pass = 0usize;
    let mut fail = 0usize;
    let mut rows: Vec<String> = Vec::new();
    let mut row = |example: &str, case: &str, ok: bool, detail: String| {
        rows.push(format!(
            "{:<16}  {:<40}  {:<4}  {}",
            example,
            case,
            if ok { "PASS" } else { "FAIL" },
            detail
        ));
        if ok {
            pass += 1;
        } else {
            fail += 1;
        }
    };

    let mut sources: Vec<_> = fs::read_dir(root.join("examples"))
        .expect("read examples/")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "iris"))
        .collect();
    sources.sort();
    assert!(!sources.is_empty(), "no examples found");

    for src in &sources {
        let name = src.file_stem().unwrap().to_string_lossy().to_string();
        let wat_path = tmp.join(format!("{}.wat", name));
        let wat2_path = tmp.join(format!("{}.2.wat", name));

        if let Err(e) = compile(iris, src, &wat_path) {
            row(&name, "compile", false, e);
            continue;
        }

        let deterministic = compile(iris, src, &wat2_path).is_ok()
            && fs::read(&wat_path).ok() == fs::read(&wat2_path).ok();
        row(
            &name,
            "determinism",
            deterministic,
            if deterministic {
                "identical WAT on recompile".into()
            } else {
                "two compiles produced different WAT".into()
            },
        );

        let wat = fs::read_to_string(&wat_path).expect("read wat");
        let exports = wat_exports(&wat);
        let source = fs::read_to_string(src).expect("read source");

        let mut tested: Vec<String> = Vec::new();
        let mut found_any = false;
        for line in source.lines() {
            let Some(ann) = line
                .trim_start()
                .strip_prefix('#')
                .map(|l| l.trim_start())
                .and_then(|l| l.strip_prefix("EXPECTED:"))
            else {
                continue;
            };
            found_any = true;
            let ann = ann.trim();
            let Some(exp) = parse_expectation(ann) else {
                row(&name, "annotation", false, format!("cannot parse: {}", ann));
                continue;
            };
            let label = format!("{}({})", exp.func, exp.args.join(", "));
            tested.push(exp.func.clone());

            if !exports.contains(&exp.func) {
                row(&name, &label, false, "not exported from module".into());
                continue;
            }

            let output = Command::new("wasmtime")
                .args(["run", "-W", "timeout=5s", "--invoke", &exp.func])
                .arg(&wat_path)
                .args(&exp.args)
                .output()
                .expect("spawn wasmtime");
            let stdout = String::from_utf8_lossy(&output.stdout);
            let got = stdout.trim();

            match exp.want {
                None => {
                    if output.status.success() {
                        row(
                            &name,
                            &label,
                            false,
                            format!("expected trap, returned {}", got),
                        );
                    } else {
                        row(&name, &label, true, "trapped as expected".into());
                    }
                }
                Some(want) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let trap = stderr
                            .lines()
                            .find(|l| {
                                let l = l.to_lowercase();
                                l.contains("trap")
                                    || l.contains("deadline")
                                    || l.contains("interrupt")
                            })
                            .unwrap_or("unknown trap")
                            .trim();
                        row(&name, &label, false, format!("trapped: {}", trap));
                    } else if got.is_empty() {
                        row(&name, &label, false, "no output from wasmtime".into());
                    } else {
                        match got.parse::<f64>() {
                            Ok(v) if v == want => row(&name, &label, true, format!("= {}", got)),
                            _ => row(
                                &name,
                                &label,
                                false,
                                format!("got {}, expected {}", got, want),
                            ),
                        }
                    }
                }
            }
        }

        if !found_any {
            row(
                &name,
                "annotations",
                false,
                "no # EXPECTED: lines in source".into(),
            );
            continue;
        }

        for export in &exports {
            if !tested.iter().any(|t| t == export) {
                row(
                    &name,
                    export,
                    false,
                    "exported but has no # EXPECTED: line".into(),
                );
            }
        }
    }

    let _ = fs::remove_dir_all(&tmp);

    println!();
    for r in &rows {
        println!("{}", r);
    }
    println!();
    println!("total: {}/{} passed, {} failed", pass, pass + fail, fail);
    assert!(fail == 0, "{} e2e case(s) failed", fail);
}
