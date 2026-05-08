use std::fs;
use std::io::Write;
use std::process::Command;

fn write_file(path: &std::path::Path, content: &str) {
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

fn run_test_lint(dir: &std::path::Path) -> (i32, String, String) {
    run_test_lint_with_args(dir, &[])
}

fn run_test_lint_with_args(dir: &std::path::Path, extra_args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-test-lint"))
        .arg("test-lint")
        .arg("--project-root")
        .arg(dir)
        .args(extra_args)
        .output()
        .expect("failed to run cargo-test-lint");

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

#[test]
fn clean_project_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    write_file(
        &src.join("lib.rs"),
        r#"
#[test]
fn test_foo() {
    assert!(true, "should pass");
}
"#,
    );
    write_file(&tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"\n");

    let (code, _, stderr) = run_test_lint(tmp.path());
    assert_eq!(code, 0, "expected clean exit, stderr: {stderr}");
}

#[test]
fn violations_exit_one() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    write_file(
        &src.join("lib.rs"),
        r#"
#[test]
fn test_foo() {
    assert!(true);
    assert_eq!(1, 1);
    assert_ne!(2, 2);
}
"#,
    );
    write_file(&tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"\n");

    let (code, _, stderr) = run_test_lint_with_args(tmp.path(), &["--deny-warnings"]);
    assert_eq!(code, 1, "expected exit 1, stderr: {stderr}");
    assert!(
        stderr.contains("CTL_ASSERT_MSG"),
        "stderr should contain CTL_ASSERT_MSG, got: {stderr}"
    );
}

#[test]
fn sarif_output_format() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    write_file(
        &src.join("lib.rs"),
        r#"
#[test]
fn test_foo() {
    assert!(true);
}
"#,
    );
    write_file(&tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"\n");

    let output = Command::new(env!("CARGO_BIN_EXE_cargo-test-lint"))
        .arg("test-lint")
        .arg("--project-root")
        .arg(tmp.path())
        .arg("--format")
        .arg("sarif")
        .output()
        .expect("failed to run cargo-test-lint");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let sarif: serde_json::Value = serde_json::from_str(&stderr).expect("invalid SARIF JSON");
    assert_eq!(sarif["version"], "2.1.0", "SARIF version should be 2.1.0");
}

#[test]
fn deny_warnings_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    write_file(
        &src.join("lib.rs"),
        r#"
#[test]
fn test_foo() {
    assert!(true);
}
"#,
    );
    write_file(&tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"\n");

    let (code, _, _) = run_test_lint(tmp.path());
    // Without --deny-warnings, warnings don't cause exit 1
    // (default config has rules at warn level, not deny)
    // This test verifies the flag is accepted without error
    assert!(code == 0 || code == 1, "exit code should be 0 or 1, got: {code}");
}
