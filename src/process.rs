//! External commands and fzf selection.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::process::{Command, Output, Stdio};

use crate::error::ZrsError;

pub fn evaluate(script: &str) -> Result<String, ZrsError> {
    let output = Command::new("zsh")
        .args(["-fc", script])
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(failed("zsh", output));
    }
    decode("zsh", output.stdout)
}

pub fn select(
    candidates: &[&str],
    query: &str,
    preview: Option<&str>,
    multi: bool,
    extra_args: &[&str],
) -> Result<Vec<String>, ZrsError> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    if candidates.iter().any(|candidate| candidate.contains('\0')) {
        return Err(ZrsError::Protocol("candidate contains a NUL byte"));
    }
    let mut command = Command::new("fzf");
    command
        .args(["--read0", "--print0", "--query", query])
        .args(extra_args)
        .env_remove("FZF_DEFAULT_OPTS")
        .env_remove("FZF_DEFAULT_OPTS_FILE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    if let Some(preview) = preview {
        command.args(["--preview", preview]);
    }
    if multi {
        command.arg("--multi");
    }
    // fzf opens the controlling terminal for interactive input. Keep its UI
    // separate from the command substitution carrying the ZLE protocol.
    match OpenOptions::new().write(true).open("/dev/tty") {
        Ok(tty) => command.stderr(tty),
        Err(_) => command.stderr(Stdio::piped()),
    };
    let mut child = command.spawn().map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => ZrsError::MissingFzf,
        _ => error.into(),
    })?;
    let mut stdin = child.stdin.take().expect("piped fzf input");
    let input = candidates.join("\0") + "\0";
    let (output, written) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(input.as_bytes()));
        (
            child.wait_with_output(),
            writer.join().expect("fzf input writer"),
        )
    });
    let output = output?;
    match output.status.code() {
        Some(1 | 130) => return Ok(Vec::new()),
        Some(0) => {}
        _ => return Err(failed("fzf", output)),
    }
    if let Err(error) = written
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    let selected = decode("fzf", output.stdout)?;
    if !selected.is_empty() && !selected.ends_with('\0') {
        return Err(ZrsError::Protocol("fzf selection is not NUL-terminated"));
    }
    Ok(selected.split_terminator('\0').map(str::to_owned).collect())
}

fn failed(program: &'static str, output: Output) -> ZrsError {
    ZrsError::Process {
        program,
        status: output.status,
        stderr: String::from_utf8_lossy(&output.stderr).trim_end().into(),
    }
}

fn decode(program: &'static str, bytes: Vec<u8>) -> Result<String, ZrsError> {
    String::from_utf8(bytes).map_err(|_| ZrsError::Encoding(program))
}
