//! Shared placeholder removal and character cursor calculation.

use std::sync::LazyLock;

use regex::Regex;

use crate::protocol::Update;

static PLACEHOLDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{[^{}\r\n]*\}\}").expect("valid placeholder regex"));

pub fn insert(left: &str, text: &str, right: &str) -> Update {
    match PLACEHOLDER.find(text) {
        Some(marker) => Update::new(
            &format!("{left}{}", &text[..marker.start()]),
            &format!("{}{right}", &text[marker.end()..]),
            true,
        ),
        None => Update::new(&format!("{left}{text}"), right, false),
    }
}

pub fn next(left: &str, right: &str) -> Option<Update> {
    let buffer = format!("{left}{right}");
    let marker = PLACEHOLDER
        .find_at(&buffer, left.len())
        .or_else(|| PLACEHOLDER.find(&buffer))?;
    Some(Update::new(
        &buffer[..marker.start()],
        &buffer[marker.end()..],
        true,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_removes_only_the_first_marker() {
        assert_eq!(
            insert("日本", "語{{}}後{{named}}", "{{right}}"),
            Update {
                buffer: "日本語後{{named}}{{right}}".into(),
                cursor: 3,
                placeholder: true,
            }
        );
    }

    #[test]
    fn next_searches_forward_then_wraps() {
        assert_eq!(
            next("前{{first}}中", "{{second}}後").unwrap(),
            Update::new("前{{first}}中", "後", true)
        );
        assert_eq!(
            next("前{{first}}中", "後").unwrap(),
            Update::new("前", "中後", true)
        );
        assert!(next("前", "後").is_none());
    }
}
