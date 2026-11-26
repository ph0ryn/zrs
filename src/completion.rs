//! Pattern-based custom fuzzy completion.

use crate::config::Completion;
use crate::error::ZrsError;
use crate::protocol::{Outcome, Update};
use crate::{process, quote};

pub fn complete(
    rules: &[Completion],
    left: &str,
    right: &str,
    word: Option<&str>,
    query: Option<&str>,
) -> Result<Outcome, ZrsError> {
    let mut matched = None;
    for rule in rules {
        if rule.pattern.is_match(left)? {
            matched = Some(rule);
            break;
        }
    }
    let Some(rule) = matched else {
        return Ok(Outcome::NoMatch);
    };
    let word = word.unwrap_or_else(|| left.rsplit(char::is_whitespace).next().unwrap_or(""));
    let query = query.unwrap_or(word);
    let prefix = left
        .strip_suffix(word)
        .ok_or(ZrsError::Protocol("word is not a buffer suffix"))?;
    let output = process::evaluate(&rule.source)?;
    let candidates = source_candidates(&output)?;
    let selected = process::select(&candidates, query, rule.preview.as_deref(), rule.multi, &[])?;
    if selected.is_empty() {
        return Ok(Outcome::NoSelection);
    }
    let replacement = selected
        .iter()
        .map(|value| quote::zsh(value))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(Outcome::Updated(Update::new(
        &format!("{prefix}{replacement}"),
        right,
        false,
    )))
}

fn source_candidates(output: &str) -> Result<Vec<&str>, ZrsError> {
    if output.contains('\0') {
        if !output.ends_with('\0') {
            return Err(ZrsError::Protocol(
                "completion source has an unterminated NUL record",
            ));
        }
        Ok(output.split_terminator('\0').collect())
    } else {
        Ok(output.lines().filter(|line| !line.is_empty()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_lines_and_nul_terminated_multiline_candidates() {
        assert_eq!(source_candidates("one\r\n\ntwo\n").unwrap(), ["one", "two"]);
        assert_eq!(
            source_candidates("one\ntwo\0日本\0").unwrap(),
            ["one\ntwo", "日本"]
        );
        assert!(source_candidates("one\0two").is_err());
    }
}
