pub mod schema;
pub mod validate;

use std::path::{Path, PathBuf};

use crate::error::ZrsError;

pub use validate::{Abbreviation, CompiledContext, Completion, Config, Snippet};

/// Resolves `$XDG_CONFIG_HOME/zrs-zsh/config.yaml` (or `~/.config/zrs-zsh/config.yaml` when unset/empty).
pub fn default_config_path() -> Result<PathBuf, ZrsError> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return Ok(PathBuf::from(xdg).join("zrs-zsh").join("config.yaml"));
    }

    let home = std::env::var_os("HOME").filter(|h| !h.is_empty());
    let Some(home) = home else {
        return Err(ZrsError::ConfigPathUnavailable);
    };

    Ok(PathBuf::from(home)
        .join(".config")
        .join("zrs-zsh")
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
    let path = default_config_path()?;
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Config::default());
        }
        Err(source) => return Err(ZrsError::ConfigRead { path, source }),
    };
    let parsed: schema::RawConfig = serde_saphyr::from_str(&raw)?;
    validate::validate_structure(parsed)
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

    #[test]
    fn rejects_invalid_abbreviation_keywords() {
        for keyword in ["git s", "a\tb", "'gs'", "\"gs\"", "\\gs"] {
            let yaml =
                format!("abbreviations:\n  - keyword: {keyword:?}\n    expand: git status\n");
            let err = parse_and_validate(&yaml).unwrap_err();
            assert!(
                matches!(err, ZrsError::ConfigValidation(_)),
                "expected validation error for {keyword:?}, got {err:?}"
            );
        }
    }

    #[test]
    fn rejects_empty_completion_preview() {
        let yaml = r#"
completions:
  - name: test
    pattern: '^test '
    source: 'printf a'
    preview: ''
"#;
        let err = parse_and_validate(yaml).unwrap_err();
        assert!(matches!(err, ZrsError::ConfigValidation(_)));
    }
}
