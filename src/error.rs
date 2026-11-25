use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ZrsError {
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
