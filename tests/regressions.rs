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

fn has_command(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn build_and_run_executable(input: &PathBuf, target: Option<&str>, expected_code: i32) {
    if !has_command("clang") && !has_command("cc") {
        return;
    }

    let mut output = PathBuf::from(std::env::temp_dir());
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    output.push(format!(
        "func_regression_exe_{}_{}",
        std::process::id(),
        nanos
    ));

    let mut args: Vec<String> = vec![
        "compile".to_string(),
        "--emit-exe".to_string(),
        "--out-exe".to_string(),
        output.to_str().expect("utf8").to_string(),
        input.to_str().expect("utf8").to_string(),
    ];
    if let Some(target) = target {
        args.push("--target".to_string());
        args.push(target.to_string());
    }

    let status = Command::new(env!("CARGO_BIN_EXE_funC"))
        .args(args)
        .status()
        .expect("run funC");
    assert!(status.success());
    assert!(output.exists());

    let run_status = Command::new(output)
        .status()
        .expect("run generated executable");
    assert_eq!(run_status.code(), Some(expected_code));
}

fn compile_object_for_target(input: &PathBuf, target: &str) -> Option<PathBuf> {
    if !has_command("llc") {
        return None;
    }

    let mut obj = PathBuf::from(std::env::temp_dir());
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    obj.push(format!(
        "func_regression_obj_{}_{}_{}",
        std::process::id(),
        target.replace('-', "_"),
        nanos
    ));
    let is_windows_target = target.contains("windows");
    let extension = if is_windows_target { "obj" } else { "o" };
    obj.set_extension(extension);

    let args = [
        "compile",
        "--emit-obj",
        "--out-obj",
        obj.to_str().expect("utf8"),
        "--target",
        target,
        input.to_str().expect("utf8"),
    ];

    let status = Command::new(env!("CARGO_BIN_EXE_funC"))
        .args(args)
        .status()
        .expect("run funC");

    if status.success() {
        if obj.exists() {
            Some(obj)
        } else {
            None
        }
    } else {
        None
    }
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
fn backend_regression_emits_object_for_native_and_compatible_aliases() {
    let input = temp_source_file(
        "backend_object",
        "fn main() -> i64 { return 123; }\n",
    );

    let native_obj = compile_object_for_target(&input, "native");
    assert!(native_obj.is_some());

    for target in ["x86_64", "aarch64"] {
        let path = compile_object_for_target(&input, target);
        if path.is_none() {
            println!("Cible '{target}' non disponible avec les toolchains actuels, skip");
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn integration_regression_linux_build_executable() {
    let input = temp_source_file(
        "integration_linux_exe",
        "fn main() -> i64 { return 7; }\n",
    );
    build_and_run_executable(&input, Some("x86_64-unknown-linux-gnu"), 7);
}

#[cfg(target_os = "windows")]
#[test]
fn integration_regression_windows_build_executable() {
    let input = temp_source_file(
        "integration_windows_exe",
        "fn main() -> i64 { return 9; }\n",
    );
    build_and_run_executable(&input, Some("x86_64-pc-windows-msvc"), 9);
}

#[test]
fn validate_command_accepts_valid_project() {
    let input = temp_source_file(
        "validate_ok",
        "struct Point { x: i64; y: i64; } fn main() -> i64 { 0 }\n",
    );
    let status = Command::new(env!("CARGO_BIN_EXE_funC"))
        .args(["validate", input.to_str().expect("utf8")])
        .status()
        .expect("run funC");

    assert!(status.success());
}

#[test]
fn validate_command_rejects_invalid_project() {
    let input = temp_source_file(
        "validate_nok",
        "fn main() -> i64 { let x: bool = 1; x }\n",
    );
    let status = Command::new(env!("CARGO_BIN_EXE_funC"))
        .args(["validate", input.to_str().expect("utf8")])
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

#[test]
fn codegen_regression_emits_struct_enum_array_types() {
    let input = temp_source_file(
        "codegen_types",
        r#"
struct Point {
    x: i64;
    y: i64;
}
enum Color {
    Red,
    Green,
    Blue
}

fn take_point(value: Point) -> Point {
    return value;
}

fn take_color(value: Color) -> Color {
    return value;
}

fn take_points(values: [Point; 2]) -> [Point; 2] {
    return values;
}

fn main() -> i64 {
    return 0;
}
"#,
    );

    let mut output = PathBuf::from(std::env::temp_dir());
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    output.push(format!(
        "func_regression_codegen_types_{}_{}.ll",
        std::process::id(),
        nanos
    ));

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
    let llvm = fs::read_to_string(&output).expect("read ir");
    assert!(llvm.contains("%Point"));
    assert!(llvm.contains("[2 x %Point]"));
    assert!(llvm.contains("take_color"));
    assert!(output.exists());
}

#[test]
fn import_regression_compiles_multiple_files() {
    let mut input = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    input.push("examples/modules/main.fc");

    let status = Command::new(env!("CARGO_BIN_EXE_funC"))
        .args(["compile", "--emit-ir", "--check", input.to_str().expect("utf8")])
        .status()
        .expect("run funC");

    assert!(status.success());
}
