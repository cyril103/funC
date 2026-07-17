use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_source_file(name: &str, contents: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    path.push(format!(
        "func_regression_{}_{}_{}.fc",
        std::process::id(),
        name,
        nanos
    ));
    fs::write(&path, contents).expect("write temp source");
    path
}

#[test]
fn parser_regression_rejects_syntax_error() {
    let input = temp_source_file(
        "parser",
        "fn main() -> i64 {\n    let x: i64 = 1\n",
    );

    let status = Command::new(env!("CARGO_BIN_EXE_funC"))
        .args(["compile", "--check", input.to_str().expect("utf8")])
        .status()
        .expect("run funC");

    assert!(!status.success());
}

#[test]
fn typecheck_regression_rejects_type_mismatch() {
    let input = temp_source_file(
        "typecheck",
        "fn main() -> i64 {\n    let x: i64 = true;\n    x\n}\n",
    );

    let status = Command::new(env!("CARGO_BIN_EXE_funC"))
        .args(["compile", "--check", input.to_str().expect("utf8")])
        .status()
        .expect("run funC");

    assert!(!status.success());
}

#[test]
fn codegen_regression_emits_llvm_ir() {
    let mut output = PathBuf::from(std::env::temp_dir());
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    output.push(format!("func_regression_codegen_{}_{}.ll", std::process::id(), nanos));

    let mut input = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    input.push("examples/if_else/basic.fc");

    let status = Command::new(env!("CARGO_BIN_EXE_funC"))
        .args([
            "compile",
            "--emit-ir",
            "--out",
            output.to_str().expect("utf8"),
            input.to_str().expect("utf8"),
        ])
        .status()
        .expect("run funC");

    assert!(status.success());
    assert!(output.exists());
    assert!(fs::metadata(&output).expect("metadata").len() > 0);
}

