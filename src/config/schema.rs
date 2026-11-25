use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawConfig {
    #[serde(default)]
    pub abbreviations: Vec<RawAbbreviation>,
    #[serde(default)]
    pub snippets: Vec<RawSnippet>,
    #[serde(default)]
    pub completions: Vec<RawCompletion>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawAbbreviation {
    pub keyword: String,
    pub expand: String,
    #[serde(default)]
    pub global: bool,
    pub context: Option<RawContext>,
    #[serde(default)]
    pub evaluate: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawContext {
    pub lbuffer: Option<String>,
    pub rbuffer: Option<String>,
    pub buffer: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawSnippet {
    pub name: String,
    pub body: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawCompletion {
    pub name: String,
    pub pattern: String,
    pub source: String,
    pub preview: Option<String>,
    #[serde(default)]
    pub multi: bool,
}
