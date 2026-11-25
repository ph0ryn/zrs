//! Quote one literal Zsh argument.

pub fn zsh(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_%,./-".contains(&byte))
    {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_whitespace_quotes_and_metacharacters() {
        assert_eq!(zsh("branch/path"), "branch/path");
        assert_eq!(zsh("日 本'\n$*"), "'日 本'\\''\n$*'");
        assert_eq!(zsh(""), "''");
    }
}
