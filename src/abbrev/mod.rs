//! Abbreviation matching and evaluation.

pub mod context;

use crate::config::Abbreviation;
use crate::error::ZrsError;
use crate::protocol::Update;
use crate::{process, template};

pub fn expand(
    abbreviations: &[Abbreviation],
    left: &str,
    right: &str,
    tokens: &[&str],
) -> Result<Option<Update>, ZrsError> {
    if left.ends_with(char::is_whitespace) {
        return Ok(None);
    }
    let Some((&word, preceding)) = tokens.split_last() else {
        return Ok(None);
    };
    let Some(prefix) = left.strip_suffix(word) else {
        return Ok(None);
    };
    if word.is_empty() || word.contains(['\'', '"', '\\']) || context::unfinished(left) {
        return Ok(None);
    }
    let matched = abbreviations.iter().find(|rule| {
        rule.keyword == word
            && match &rule.context {
                Some(ctx) => {
                    ctx.lbuffer
                        .as_ref()
                        .is_none_or(|pattern| pattern.is_match(prefix))
                        && ctx
                            .rbuffer
                            .as_ref()
                            .is_none_or(|pattern| pattern.is_match(right))
                        && ctx
                            .buffer
                            .as_ref()
                            .is_none_or(|pattern| pattern.is_match(&format!("{left}{right}")))
                }
                None => rule.global || context::command_position(preceding),
            }
    });
    let Some(rule) = matched else {
        return Ok(None);
    };
    let evaluated;
    let replacement = if rule.evaluate {
        evaluated = process::evaluate(&rule.expand)?;
        evaluated.trim_end_matches('\n')
    } else {
        &rule.expand
    };
    Ok(Some(template::insert(prefix, replacement, right)))
}
