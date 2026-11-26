use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::os::unix::fs::PermissionsExt;
use tempfile::{NamedTempFile, TempDir};

#[test]
fn placeholder_returns_buffer_and_character_cursor() {
    cargo_bin_cmd!("zrs-zsh")
        .args(["placeholder", "--lbuffer=日本", "--rbuffer={{word}}残り"])
        .assert()
        .success()
        .stdout(concat!("placeholder\0日本残り\0", "2\0"));
}

#[test]
fn init_prints_zsh_script() {
    cargo_bin_cmd!("zrs-zsh")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("_zrs_toggle"));
}

#[test]
fn init_installs_history_binding_only_with_keybinds() {
    cargo_bin_cmd!("zrs-zsh")
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("'^R' _zrs_history"));
    cargo_bin_cmd!("zrs-zsh")
        .args(["init", "--no-keybinds"])
        .assert()
        .success()
        .stdout(predicate::str::contains("zle -N _zrs_history"))
        .stdout(predicate::str::contains("\n_zrs_bind_defaults\n").not());
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

    cargo_bin_cmd!("zrs-zsh")
        .args(["validate", "--config"])
        .arg(file.path())
        .assert()
        .success();
}

#[test]
fn validate_rejects_invalid_config() {
    let file = NamedTempFile::new().unwrap();
    std::fs::write(file.path(), "unknown_field: 1\n").unwrap();

    cargo_bin_cmd!("zrs-zsh")
        .args(["validate", "--config"])
        .arg(file.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("zrs-zsh:"));
}

fn xdg_config(yaml: &str) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("zrs-zsh");
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("config.yaml"), yaml).unwrap();
    dir
}

fn mock_fzf(command: &mut assert_cmd::Command) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let fzf = dir.path().join("fzf");
    std::fs::write(
        &fzf,
        r#"#!/bin/sh
set -eu
printf '%s\n' "$@" > "$FZF_ARGS"
cat > "$FZF_INPUT"
printf invoked > "$FZF_MARKER"
if [ "${FZF_CANCEL:-0}" = 1 ]; then
  exit 130
fi
printf '%s\0' "$FZF_SELECTION"
"#,
    )
    .unwrap();
    std::fs::set_permissions(&fzf, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut paths = vec![dir.path().to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    command
        .env("PATH", std::env::join_paths(paths).unwrap())
        .env("FZF_ARGS", dir.path().join("args"))
        .env("FZF_INPUT", dir.path().join("input"))
        .env("FZF_MARKER", dir.path().join("invoked"));
    dir
}

#[test]
fn history_selects_with_initial_query_and_index_tiebreak() {
    let selected = "printf '日本 {{value}}'\nnext line";
    let mut command = cargo_bin_cmd!("zrs-zsh");
    let fzf = mock_fzf(&mut command);
    command
        .env("FZF_SELECTION", selected)
        .args(["history", "--lbuffer=print 日", "--rbuffer=old suffix"])
        .write_stdin("old command\0printf '日本 {{value}}'\nnext line\0")
        .assert()
        .success()
        .stdout(concat!(
            "normal\0printf '日本 {{value}}'\nnext line\0",
            "31\0"
        ));

    let args = std::fs::read_to_string(fzf.path().join("args")).unwrap();
    assert_eq!(
        args,
        "--read0\n--print0\n--query\nprint 日\n--tiebreak=index\n"
    );
    assert_eq!(
        std::fs::read(fzf.path().join("input")).unwrap(),
        "old command\0printf '日本 {{value}}'\nnext line\0".as_bytes()
    );
}

#[test]
fn history_cancellation_and_empty_input_preserve_the_buffer() {
    let mut command = cargo_bin_cmd!("zrs-zsh");
    let fzf = mock_fzf(&mut command);
    command
        .env("FZF_CANCEL", "1")
        .args(["history", "--lbuffer=query", "--rbuffer=suffix"])
        .write_stdin("newest\0oldest\0")
        .assert()
        .code(3)
        .stdout("");
    assert_eq!(
        std::fs::read_to_string(fzf.path().join("args")).unwrap(),
        "--read0\n--print0\n--query\nquery\n--tiebreak=index\n"
    );

    let mut empty_command = cargo_bin_cmd!("zrs-zsh");
    let empty_fzf = mock_fzf(&mut empty_command);
    empty_command
        .args(["history", "--lbuffer=", "--rbuffer="])
        .write_stdin(" \n\t\0")
        .assert()
        .code(3)
        .stdout("");
    assert!(!empty_fzf.path().join("invoked").exists());
}

#[test]
fn history_ignores_missing_and_invalid_config() {
    for config in [None, Some("unknown_field: [\n")] {
        let dir = tempfile::tempdir().unwrap();
        if let Some(yaml) = config {
            std::fs::create_dir(dir.path().join("zrs-zsh")).unwrap();
            std::fs::write(dir.path().join("zrs-zsh/config.yaml"), yaml).unwrap();
        }
        let mut command = cargo_bin_cmd!("zrs-zsh");
        let _fzf = mock_fzf(&mut command);
        command
            .env("XDG_CONFIG_HOME", dir.path())
            .env("FZF_SELECTION", "echo selected")
            .args(["history", "--lbuffer=", "--rbuffer="])
            .write_stdin("echo selected\0")
            .assert()
            .success()
            .stdout(concat!("normal\0echo selected\0", "13\0"));
    }
}

#[test]
fn snippet_reports_missing_fzf() {
    let config = xdg_config("snippets: [{name: example, body: example}]");
    cargo_bin_cmd!("zrs-zsh")
        .args(["snippet", "--lbuffer=", "--rbuffer="])
        .env("XDG_CONFIG_HOME", config.path())
        .env("PATH", config.path())
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("install fzf"));
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
    expand: wrong branch
    context:
      buffer: ' B --force$'
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
        cargo_bin_cmd!("zrs-zsh")
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
        cargo_bin_cmd!("zrs-zsh")
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
    cargo_bin_cmd!("zrs-zsh")
        .env("XDG_CONFIG_HOME", config.path())
        .args(["expand", "--lbuffer=date", "--rbuffer="])
        .write_stdin("date\0")
        .assert()
        .success()
        .stdout(concat!("normal\0today\0", "5\0"));
    cargo_bin_cmd!("zrs-zsh")
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
        cargo_bin_cmd!("zrs-zsh")
            .env("XDG_CONFIG_HOME", config.path())
            .args([subcommand, "--lbuffer=-x", "--rbuffer="])
            .write_stdin("-x\0")
            .assert()
            .code(2)
            .stdout("")
            .stderr("");
    }
    cargo_bin_cmd!("zrs-zsh")
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
        cargo_bin_cmd!("zrs-zsh")
            .env("XDG_CONFIG_HOME", config.path())
            .args(["complete", &format!("--lbuffer={left}"), "--rbuffer="])
            .assert()
            .code(code)
            .stdout("");
    }
}

#[test]
fn expand_accepts_positional_tokens_including_hyphen_prefixed_values() {
    let config = xdg_config(
        r#"
abbreviations:
  - keyword: gs
    expand: git status
  - keyword: -x
    expand: --extended
    global: true
"#,
    );
    cargo_bin_cmd!("zrs-zsh")
        .env("XDG_CONFIG_HOME", config.path())
        .args([
            "expand",
            "--lbuffer=sudo -u root gs",
            "--rbuffer=",
            "--",
            "sudo",
            "-u",
            "root",
            "gs",
        ])
        .assert()
        .success()
        .stdout(concat!("normal\0sudo -u root git status\0", "23\0"));

    cargo_bin_cmd!("zrs-zsh")
        .env("XDG_CONFIG_HOME", config.path())
        .args([
            "expand",
            "--lbuffer=cmd -x",
            "--rbuffer=",
            "--",
            "cmd",
            "-x",
        ])
        .assert()
        .success()
        .stdout(concat!("normal\0cmd --extended\0", "14\0"));
