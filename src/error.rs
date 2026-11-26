use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ZrsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("install fzf and make it available on PATH to select candidates")]
    MissingFzf,

    #[error("{0} returned invalid UTF-8")]
    Encoding(&'static str),

    #[error("{program} failed ({status}): {stderr}")]
    Process {
        program: &'static str,
        status: std::process::ExitStatus,
        stderr: String,
    },

    #[error("invalid ZLE protocol: {0}")]
    Protocol(&'static str),
    #[error("failed to determine config path: neither XDG_CONFIG_HOME nor HOME is set")]
    ConfigPathUnavailable,

    #[error("failed to read config file {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse YAML config: {0}")]
    ConfigParse(#[from] serde_saphyr::Error),

    #[error("invalid config: {0}")]
    ConfigValidation(String),

    #[error("invalid regex in {field}: {source}")]
    InvalidRegex {
        field: String,
        #[source]
        source: regex::Error,
    },
}
