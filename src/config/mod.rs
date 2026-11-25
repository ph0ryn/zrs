pub mod schema;
pub mod validate;

use std::path::{Path, PathBuf};

use crate::error::ZrsError;

pub use validate::{Abbreviation, CompiledContext, Completion, Config, Snippet};

/// Resolves `$XDG_CONFIG_HOME/zrs/config.yaml` (or `~/.config/zrs/config.yaml` when unset/empty).
pub fn default_config_path() -> Result<PathBuf, ZrsError> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return Ok(PathBuf::from(xdg).join("zrs").join("config.yaml"));
    }

    let home = std::env::var_os("HOME").filter(|h| !h.is_empty());
    let Some(home) = home else {
        return Err(ZrsError::ConfigPathUnavailable);
    };

    Ok(PathBuf::from(home)
        .join(".config")
        .join("zrs")
        .join("config.yaml"))
}

pub fn load_from_path(path: &Path) -> Result<Config, ZrsError> {
    let raw = std::fs::read_to_string(path).map_err(|source| ZrsError::ConfigRead {
        path: path.to_path_buf(),
        source,
    })?;
    parse_and_validate(&raw)
}

pub fn load_interactive() -> Result<Config, ZrsError> {
    match load_from_path(&default_config_path()?) {
        Err(ZrsError::ConfigRead { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(Config::default())
        }
        result => result,
    }
}

pub fn parse_and_validate(yaml: &str) -> Result<Config, ZrsError> {
    let raw: schema::RawConfig = serde_saphyr::from_str(yaml)?;
    validate::validate(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_sections_default_to_empty() {
        let cfg = parse_and_validate("{}").expect("empty mapping should be valid");
        assert!(cfg.abbreviations.is_empty());
        assert!(cfg.snippets.is_empty());
        assert!(cfg.completions.is_empty());
    }

    #[test]
    fn rejects_unknown_fields() {
        let err = parse_and_validate("unknown_key: true").unwrap_err();
        assert!(matches!(err, ZrsError::ConfigParse(_)));
    }

    #[test]
    fn rejects_global_and_context_together() {
        let yaml = r#"
abbreviations:
  - keyword: B
    expand: main
    global: true
    context:
      lbuffer: '^git\s+'
"#;
        let err = parse_and_validate(yaml).unwrap_err();
        assert!(matches!(err, ZrsError::ConfigValidation(_)));
    }
}
