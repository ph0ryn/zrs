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
    let mut combined = None;
    let mut matched = None;
    for rule in abbreviations {
        if rule.keyword != word {
            continue;
        }
        let is_match = match &rule.context {
            Some(ctx) => {
                let lbuf_ok = match &ctx.lbuffer {
                    Some(pattern) => pattern.is_match(prefix)?,
                    None => true,
                };
                let rbuf_ok = lbuf_ok
                    && match &ctx.rbuffer {
                        Some(pattern) => pattern.is_match(right)?,
                        None => true,
                    };
                rbuf_ok
                    && match &ctx.buffer {
                        Some(pattern) => {
                            let buffer = if right.is_empty() {
                                left
                            } else {
                                combined.get_or_insert_with(|| format!("{left}{right}"))
                            };
                            pattern.is_match(buffer)?
                        }
                        None => true,
                    }
            }
            None => rule.global || context::command_position(preceding),
        };
        if is_match {
            matched = Some(rule);
            break;
        }
    }
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
