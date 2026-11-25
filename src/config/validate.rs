use regex::Regex;

use crate::config::schema::{RawAbbreviation, RawCompletion, RawConfig, RawContext, RawSnippet};
use crate::error::ZrsError;

#[derive(Debug)]
pub struct Config {
    pub abbreviations: Vec<Abbreviation>,
    pub snippets: Vec<Snippet>,
    pub completions: Vec<Completion>,
}

#[derive(Debug)]
pub struct Abbreviation {
    pub keyword: String,
    pub expand: String,
    pub global: bool,
    pub context: Option<CompiledContext>,
    pub evaluate: bool,
}

#[derive(Debug)]
pub struct CompiledContext {
    pub lbuffer: Option<Regex>,
    pub rbuffer: Option<Regex>,
    pub buffer: Option<Regex>,
}

#[derive(Debug)]
pub struct Snippet {
    pub name: String,
    pub body: String,
}

#[derive(Debug)]
pub struct Completion {
    pub name: String,
    pub pattern: Regex,
    pub source: String,
    pub preview: Option<String>,
    pub multi: bool,
}

pub fn validate(raw: RawConfig) -> Result<Config, ZrsError> {
    let abbreviations = raw
        .abbreviations
        .into_iter()
        .enumerate()
        .map(|(i, item)| validate_abbreviation(i, item))
        .collect::<Result<Vec<_>, _>>()?;

    let snippets = raw
        .snippets
        .into_iter()
        .enumerate()
        .map(|(i, item)| validate_snippet(i, item))
        .collect::<Result<Vec<_>, _>>()?;

    let completions = raw
        .completions
        .into_iter()
        .enumerate()
        .map(|(i, item)| validate_completion(i, item))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Config {
        abbreviations,
        snippets,
        completions,
    })
}

fn validate_abbreviation(index: usize, raw: RawAbbreviation) -> Result<Abbreviation, ZrsError> {
    if raw.keyword.is_empty() {
        return Err(ZrsError::ConfigValidation(format!(
            "abbreviations[{index}].keyword must not be empty"
        )));
    }
    if raw.global && raw.context.is_some() {
        return Err(ZrsError::ConfigValidation(format!(
            "abbreviations[{index}] cannot specify both global: true and context"
        )));
    }

    let context = raw
        .context
        .map(|ctx| compile_context(index, ctx))
        .transpose()?;

    Ok(Abbreviation {
        keyword: raw.keyword,
        expand: raw.expand,
        global: raw.global,
        context,
        evaluate: raw.evaluate,
    })
}

fn compile_context(index: usize, raw: RawContext) -> Result<CompiledContext, ZrsError> {
    if raw.lbuffer.is_none() && raw.rbuffer.is_none() && raw.buffer.is_none() {
        return Err(ZrsError::ConfigValidation(format!(
            "abbreviations[{index}].context must specify at least one of lbuffer, rbuffer, or buffer"
        )));
    }

    Ok(CompiledContext {
        lbuffer: raw
            .lbuffer
            .map(|p| compile_regex(&format!("abbreviations[{index}].context.lbuffer"), &p))
            .transpose()?,
        rbuffer: raw
            .rbuffer
            .map(|p| compile_regex(&format!("abbreviations[{index}].context.rbuffer"), &p))
            .transpose()?,
        buffer: raw
            .buffer
            .map(|p| compile_regex(&format!("abbreviations[{index}].context.buffer"), &p))
            .transpose()?,
    })
}

fn validate_snippet(index: usize, raw: RawSnippet) -> Result<Snippet, ZrsError> {
    if raw.name.is_empty() {
        return Err(ZrsError::ConfigValidation(format!(
            "snippets[{index}].name must not be empty"
        )));
    }
    Ok(Snippet {
        name: raw.name,
        body: raw.body,
    })
}

fn validate_completion(index: usize, raw: RawCompletion) -> Result<Completion, ZrsError> {
    if raw.name.is_empty() {
        return Err(ZrsError::ConfigValidation(format!(
            "completions[{index}].name must not be empty"
        )));
    }
    if raw.source.is_empty() {
        return Err(ZrsError::ConfigValidation(format!(
            "completions[{index}].source must not be empty"
        )));
    }
    let pattern = compile_regex(&format!("completions[{index}].pattern"), &raw.pattern)?;
    Ok(Completion {
        name: raw.name,
        pattern,
        source: raw.source,
        preview: raw.preview,
        multi: raw.multi,
    })
}

fn compile_regex(field: &str, pattern: &str) -> Result<Regex, ZrsError> {
    Regex::new(pattern).map_err(|source| ZrsError::InvalidRegex {
        field: field.to_string(),
        source,
    })
}
