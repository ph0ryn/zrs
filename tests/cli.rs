use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::NamedTempFile;

#[test]
fn init_prints_zsh_script() {
    cargo_bin_cmd!("zrs")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("_zrs_toggle"));
}

#[test]
fn validate_accepts_valid_config() {
    let file = NamedTempFile::new().unwrap();
    std::fs::write(
        file.path(),
        r#"
abbreviations:
  - keyword: gs
    expand: git status --short --branch
"#,
    )
    .unwrap();

    cargo_bin_cmd!("zrs")
        .args(["validate", "--config"])
        .arg(file.path())
        .assert()
        .success();
}

#[test]
fn validate_rejects_invalid_config() {
    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "unknown_field: 1\n").unwrap();

    cargo_bin_cmd!("zrs")
        .args(["validate", "--config"])
        .arg(file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("zrs:"));
}
