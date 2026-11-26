use std::cell::OnceCell;
use std::collections::HashMap;

use regex::Regex;

use crate::config::schema::{RawAbbreviation, RawCompletion, RawConfig, RawContext, RawSnippet};
use crate::error::ZrsError;

#[derive(Debug, Default)]
pub struct Config {
    pub abbreviations: Vec<Abbreviation>,
    pub snippets: Vec<Snippet>,
    pub completions: Vec<Completion>,
}

#[derive(Debug)]
pub struct LazyRegex {
    pattern: String,
    field: String,
    compiled: OnceCell<Regex>,
}

impl LazyRegex {
    fn new(pattern: String, field: String) -> Self {
        Self {
            pattern,
            field,
            compiled: OnceCell::new(),
        }
    }

    pub fn is_match(&self, text: &str) -> Result<bool, ZrsError> {
        Ok(self.compile()?.is_match(text))
    }

    fn compile(&self) -> Result<&Regex, ZrsError> {
        if let Some(regex) = self.compiled.get() {
            return Ok(regex);
        }
        let regex = Regex::new(&self.pattern).map_err(|source| ZrsError::InvalidRegex {
            field: self.field.clone(),
            source,
        })?;
        Ok(self.compiled.get_or_init(|| regex))
    }

    fn compile_with_cache(&self, patterns: &mut HashMap<String, Regex>) -> Result<(), ZrsError> {
        if self.compiled.get().is_some() {
            return Ok(());
        }
        let regex = compile_regex(patterns, &self.pattern, &self.field)?;
        let _ = self.compiled.set(regex);
        Ok(())
    }
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
    pub lbuffer: Option<LazyRegex>,
    pub rbuffer: Option<LazyRegex>,
    pub buffer: Option<LazyRegex>,
}

#[derive(Debug)]
pub struct Snippet {
    pub name: String,
    pub body: String,
}

#[derive(Debug)]
pub struct Completion {
    pub name: String,
    pub pattern: LazyRegex,
    pub source: String,
    pub preview: Option<String>,
    pub multi: bool,
}

pub fn validate_structure(raw: RawConfig) -> Result<Config, ZrsError> {
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

pub fn validate(raw: RawConfig) -> Result<Config, ZrsError> {
    let config = validate_structure(raw)?;
    let mut patterns = HashMap::new();
    for abbr in &config.abbreviations {
        if let Some(ctx) = &abbr.context {
            for regex in [&ctx.lbuffer, &ctx.rbuffer, &ctx.buffer]
                .into_iter()
                .flatten()
            {
                regex.compile_with_cache(&mut patterns)?;
            }
        }
    }
    for completion in &config.completions {
        completion.pattern.compile_with_cache(&mut patterns)?;
    }
    Ok(config)
}

fn validate_abbreviation(index: usize, raw: RawAbbreviation) -> Result<Abbreviation, ZrsError> {
    if raw.keyword.is_empty() {
        return Err(ZrsError::ConfigValidation(format!(
            "abbreviations[{index}].keyword must not be empty"
        )));
    }
    if raw.keyword.chars().any(char::is_whitespace) {
        return Err(ZrsError::ConfigValidation(format!(
            "abbreviations[{index}].keyword must not contain whitespace"
        )));
    }
    if raw.keyword.contains(['\'', '"', '\\']) {
        return Err(ZrsError::ConfigValidation(format!(
            "abbreviations[{index}].keyword must not contain quotes or backslashes"
        )));
    }
    if raw.global && raw.context.is_some() {
        return Err(ZrsError::ConfigValidation(format!(
            "abbreviations[{index}] cannot specify both global: true and context"
        )));
    }

    let context = raw
        .context
        .map(|ctx| validate_context(index, ctx))
        .transpose()?;

    Ok(Abbreviation {
        keyword: raw.keyword,
        expand: raw.expand,
        global: raw.global,
        context,
        evaluate: raw.evaluate,
    })
}

fn validate_context(index: usize, raw: RawContext) -> Result<CompiledContext, ZrsError> {
    if raw.lbuffer.is_none() && raw.rbuffer.is_none() && raw.buffer.is_none() {
        return Err(ZrsError::ConfigValidation(format!(
            "abbreviations[{index}].context must specify at least one of lbuffer, rbuffer, or buffer"
        )));
    }

    let lazy = |pattern: Option<String>, field| {
        pattern.map(|pattern| {
            LazyRegex::new(pattern, format!("abbreviations[{index}].context.{field}"))
        })
    };
    Ok(CompiledContext {
        lbuffer: lazy(raw.lbuffer, "lbuffer"),
        rbuffer: lazy(raw.rbuffer, "rbuffer"),
        buffer: lazy(raw.buffer, "buffer"),
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
    if raw.preview.as_deref() == Some("") {
        return Err(ZrsError::ConfigValidation(format!(
            "completions[{index}].preview must not be empty"
        )));
    }
    let pattern = LazyRegex::new(raw.pattern, format!("completions[{index}].pattern"));
    Ok(Completion {
        name: raw.name,
        pattern,
        source: raw.source,
        preview: raw.preview,
        multi: raw.multi,
    })
}

fn compile_regex(
    patterns: &mut HashMap<String, Regex>,
    pattern: &str,
    field: &str,
) -> Result<Regex, ZrsError> {
    if let Some(regex) = patterns.get(pattern) {
        return Ok(regex.clone());
    }
    let regex = Regex::new(pattern).map_err(|source| ZrsError::InvalidRegex {
        field: field.to_owned(),
        source,
    })?;
    patterns.insert(pattern.to_owned(), regex.clone());
    Ok(regex)
}
