//! Interactive snippet selection.

use crate::config::Snippet;
use crate::error::ZrsError;
use crate::protocol::Outcome;
use crate::{process, template};

pub fn select(snippets: &[Snippet], left: &str, right: &str) -> Result<Outcome, ZrsError> {
    if snippets.is_empty() {
        return Ok(Outcome::NoMatch);
    }
    let labels: Vec<_> = snippets
        .iter()
        .enumerate()
        .map(|(index, snippet)| format!("{index}\t{}", snippet.name))
        .collect();
    let candidates: Vec<_> = labels.iter().map(String::as_str).collect();
    let selected = process::select(
        &candidates,
        "",
        None,
        false,
        &["--delimiter=\t", "--with-nth=2.."],
    )?;
    let Some(label) = selected.first() else {
        return Ok(Outcome::NoSelection);
    };
    let index = labels
        .iter()
        .position(|candidate| candidate == label)
        .ok_or(ZrsError::Protocol("fzf returned an unknown snippet"))?;
    Ok(Outcome::Updated(template::insert(
        left,
        &snippets[index].body,
        right,
    )))
}
