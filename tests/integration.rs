use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn gundi() -> Command {
    Command::cargo_bin("gundi").unwrap()
}

/// Set up a temp dir with a git repo and some files with TODOs.
fn setup_git_repo() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Init git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create a file with debt comments
    let src = dir.path().join("main.rs");
    fs::write(
        &src,
        r#"fn main() {
    // TODO: implement proper error handling
    println!("hello");
    // FIXME: this is broken
    let x = 1;
    // HACK: workaround for upstream bug
}
"#,
    )
    .unwrap();

    // Commit it
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();

    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    dir
}

fn setup_plain_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("test.py");
    fs::write(
        &src,
        "# TODO: refactor this function\n# BUG: off by one error\n# XXX: temporary\n",
    )
    .unwrap();
    dir
}

#[test]
fn test_version_flag() {
    gundi()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("gundi"));
}

#[test]
fn test_help_flag() {
    gundi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO scanner"));
}

#[test]
fn test_scan_no_blame() {
    let dir = setup_plain_dir();

    gundi()
        .args(["--no-blame", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"))
        .stdout(predicate::str::contains("BUG"))
        .stdout(predicate::str::contains("XXX"));
}

#[test]
fn test_scan_with_git_blame() {
    let dir = setup_git_repo();

    gundi()
        .arg(dir.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"))
        .stdout(predicate::str::contains("FIXME"))
        .stdout(predicate::str::contains("HACK"))
        .stdout(predicate::str::contains("Test User"));
}

#[test]
fn test_json_output() {
    let dir = setup_plain_dir();

    let output = gundi()
        .args(["--no-blame", "--json", dir.path().to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.len(), 3);
}

#[test]
fn test_markdown_output() {
    let dir = setup_plain_dir();

    gundi()
        .args(["--no-blame", "--md", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("| Type |"))
        .stdout(predicate::str::contains("**Total: 3 items**"));
}

#[test]
fn test_type_filter() {
    let dir = setup_plain_dir();

    gundi()
        .args(["--no-blame", "--type", "todo", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"))
        .stdout(predicate::str::contains("Total: 1 items"));
}

#[test]
fn test_author_filter() {
    let dir = setup_git_repo();

    gundi()
        .args(["--author", "Test User", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Test User"));
}

#[test]
fn test_summary_mode() {
    let dir = setup_plain_dir();

    gundi()
        .args(["--no-blame", "--summary", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Code Debt Summary: 3 total items"))
        .stdout(predicate::str::contains("By Type:"));
}

#[test]
fn test_summary_json() {
    let dir = setup_plain_dir();

    let output = gundi()
        .args([
            "--no-blame",
            "--summary",
            "--json",
            dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["total"], 3);
}

#[test]
fn test_fail_on_gate() {
    let dir = setup_git_repo();

    // With --fail-on 0, any item with age > 0 should fail
    gundi()
        .args(["--fail-on", "0", dir.path().to_str().unwrap()])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("CI gate"));
}

#[test]
fn test_fail_on_no_trigger() {
    let dir = setup_git_repo();

    // With --fail-on 99999, nothing should be old enough
    gundi()
        .args(["--fail-on", "99999", dir.path().to_str().unwrap()])
        .assert()
        .success();
}

#[test]
fn test_nonexistent_path() {
    gundi()
        .arg("/nonexistent/path/that/does/not/exist")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_empty_directory() {
    let dir = TempDir::new().unwrap();

    gundi()
        .args(["--no-blame", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("No code debt found"));
}

#[test]
fn test_clean_codebase() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("clean.rs");
    fs::write(&file, "fn main() {\n    println!(\"hello\");\n}\n").unwrap();

    gundi()
        .args(["--no-blame", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("No code debt found"));
}

#[test]
fn test_verbose_reports_scan_progress() {
    let dir = setup_plain_dir();

    gundi()
        .args(["--verbose", "--no-blame", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::contains("scanning"))
        .stderr(predicate::str::contains("scan complete"));
}

#[test]
fn test_quiet_with_fail_on_suppresses_output() {
    let dir = setup_git_repo();

    // --quiet + --fail-on triggered: no stdout, no stderr message, just exit code 1
    gundi()
        .args(["--quiet", "--fail-on", "0", dir.path().to_str().unwrap()])
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_verbose_and_quiet_are_mutually_exclusive() {
    let dir = setup_plain_dir();

    gundi()
        .args([
            "--verbose",
            "--quiet",
            "--no-blame",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .failure();
}

#[test]
fn test_unicode_content_does_not_panic() {
    // Regression test: byte-slicing in the table formatter previously panicked
    // on multi-byte UTF-8 boundaries.
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("unicode.rs");
    fs::write(
        &file,
        "// TODO: 修复这个非常非常长的注释问题 — handle αβγ and 🦀 properly\n",
    )
    .unwrap();

    gundi()
        .args(["--no-blame", dir.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("TODO"))
        .stdout(predicate::str::contains("Total: 1 items"));
}

#[test]
fn test_path_is_file_not_directory() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("single.rs");
    fs::write(&file, "// TODO: x\n").unwrap();

    gundi()
        .arg(file.to_str().unwrap())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not a directory"));
}
