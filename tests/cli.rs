use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn placeholder_returns_buffer_and_character_cursor() {
    cargo_bin_cmd!("zrs")
        .args(["placeholder", "--lbuffer=日本", "--rbuffer={{word}}残り"])
        .assert()
        .success()
        .stdout(concat!("placeholder\0日本残り\0", "2\0"));
}

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

fn xdg_config(yaml: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("zrs");
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("config.yaml"), yaml).unwrap();
    dir
}

#[test]
fn abbreviation_rules_respect_position_order_context_and_guards() {
    let config = xdg_config(
        r#"
abbreviations:
  - keyword: gs
    expand: git status
  - keyword: gs
    expand: '{{}}global'
    global: true
  - keyword: B
    expand: branch
    context:
      lbuffer: '^git switch $'
      rbuffer: '^ --quiet$'
      buffer: ' B '
"#,
    );
    for (left, right, tokens, expected) in [
        (
            "echo x; gs",
            "",
            "echo\0x\0;\0gs\0",
            concat!("normal\0echo x; git status\0", "18\0"),
        ),
        (
            "echo gs",
            "",
            "echo\0gs\0",
            concat!("placeholder\0echo global\0", "5\0"),
        ),
        (
            "git switch B",
            " --quiet",
            "git\0switch\0B\0",
            concat!("normal\0git switch branch --quiet\0", "17\0"),
        ),
    ] {
        cargo_bin_cmd!("zrs")
            .env("XDG_CONFIG_HOME", config.path())
            .args([
                "expand",
                &format!("--lbuffer={left}"),
                &format!("--rbuffer={right}"),
            ])
            .write_stdin(tokens)
            .assert()
            .success()
            .stdout(expected);
    }
    for (left, tokens) in [
        ("gs ", "gs\0"),
        ("echo 'gs", "echo\0gs\0"),
        ("echo 'gs'", "echo\0'gs'\0"),
        ("echo \\gs", "echo\0\\gs\0"),
        ("echo $(foo gs", "echo\0gs\0"),
        ("git switch B", "git\0switch\0B\0"),
    ] {
        cargo_bin_cmd!("zrs")
            .env("XDG_CONFIG_HOME", config.path())
            .args(["expand", &format!("--lbuffer={left}"), "--rbuffer="])
            .write_stdin(tokens)
            .assert()
            .code(2)
            .stdout("");
    }
}

#[test]
fn evaluates_matching_abbreviations_and_propagates_command_failure() {
    let config = xdg_config(
        r#"
abbreviations:
  - keyword: date
    evaluate: true
    expand: "printf 'today\\n\\n'"
  - keyword: fail
    evaluate: true
    expand: "print -ru2 -- 'failed on purpose'; exit 7"
"#,
    );
    cargo_bin_cmd!("zrs")
        .env("XDG_CONFIG_HOME", config.path())
        .args(["expand", "--lbuffer=date", "--rbuffer="])
        .write_stdin("date\0")
        .assert()
        .success()
        .stdout(concat!("normal\0today\0", "5\0"));
    cargo_bin_cmd!("zrs")
        .env("XDG_CONFIG_HOME", config.path())
        .args(["expand", "--lbuffer=fail", "--rbuffer="])
        .write_stdin("fail\0")
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("failed on purpose"));
}

#[test]
fn missing_config_is_allowed_only_for_interactive_operations() {
    let config = tempfile::tempdir().unwrap();
    for subcommand in ["expand", "snippet", "complete"] {
        cargo_bin_cmd!("zrs")
            .env("XDG_CONFIG_HOME", config.path())
            .args([subcommand, "--lbuffer=-x", "--rbuffer="])
            .write_stdin("-x\0")
            .assert()
            .code(2)
            .stdout("")
            .stderr("");
    }
    cargo_bin_cmd!("zrs")
        .env("XDG_CONFIG_HOME", config.path())
        .arg("validate")
        .assert()
        .code(1);
}

#[test]
fn completion_distinguishes_no_rule_no_candidates_and_source_failure() {
    let config = xdg_config(
        r#"
completions:
  - name: empty
    pattern: '^empty '
    source: ':'
  - name: fail
    pattern: '^fail '
    source: 'exit 7'
"#,
    );
    for (left, code) in [("other ", 2), ("empty ", 3), ("fail ", 1)] {
        cargo_bin_cmd!("zrs")
            .env("XDG_CONFIG_HOME", config.path())
            .args(["complete", &format!("--lbuffer={left}"), "--rbuffer="])
            .assert()
            .code(code)
            .stdout("");
    }
}
