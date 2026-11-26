//! Interactive selection from Zsh's command history.

use crate::error::ZrsError;
use crate::process;
use crate::protocol::{Outcome, Update};

pub fn select(input: &[u8], lbuffer: &str, _rbuffer: &str) -> Result<Outcome, ZrsError> {
    let candidates = parse_candidates(input)?;
    if candidates.is_empty() {
        return Ok(Outcome::NoSelection);
    }
    let selected = process::select(&candidates, lbuffer, None, false, &["--tiebreak=index"])?;
    Ok(selection_outcome(selected))
}

fn parse_candidates(input: &[u8]) -> Result<Vec<&str>, ZrsError> {
    if !input.is_empty() && !input.ends_with(b"\0") {
        return Err(ZrsError::Protocol("history input is not NUL-terminated"));
    }
    Ok(input
        .split(|byte| *byte == 0)
        .filter_map(|record| std::str::from_utf8(record).ok())
        .filter(|entry| !entry.trim().is_empty())
        .collect())
}

fn selection_outcome(selected: Vec<String>) -> Outcome {
    selected
        .into_iter()
        .next()
        .map_or(Outcome::NoSelection, |selected| {
            Outcome::Updated(Update::new(&selected, "", false))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn updated(outcome: Outcome) -> Update {
        let Outcome::Updated(update) = outcome else {
            panic!("expected a buffer update");
        };
        update
    }

    #[test]
    fn preserves_duplicate_entries_in_order() {
        assert_eq!(
            parse_candidates(b"echo one\0echo two\0echo one\0").unwrap(),
            ["echo one", "echo two", "echo one"]
        );
    }

    #[test]
    fn filters_empty_and_whitespace_only_entries() {
        assert_eq!(
            parse_candidates(b"\0 \n\t\0echo kept\0\0").unwrap(),
            ["echo kept"]
        );
        assert!(parse_candidates(b"\0 \n\t\0").unwrap().is_empty());
    }

    #[test]
    fn preserves_multiline_commands_as_single_candidates() {
        assert_eq!(
            parse_candidates(b"printf 'one\ntwo'\nprintf 'three'\0").unwrap(),
            ["printf 'one\ntwo'\nprintf 'three'"]
        );
    }

    #[test]
    fn skips_invalid_utf8_entries() {
        assert_eq!(
            parse_candidates(b"echo valid\0\xff\0echo \xe6\x97\xa5\xe6\x9c\xac\0").unwrap(),
            ["echo valid", "echo 日本"]
        );
    }

    #[test]
    fn rejects_unterminated_input() {
        assert!(matches!(
            parse_candidates(b"echo one\0echo two"),
            Err(ZrsError::Protocol("history input is not NUL-terminated"))
        ));
    }

    #[test]
    fn selection_replaces_the_full_buffer_without_template_or_quote_processing() {
        let selected = "printf '日本 {{value}}'";
        let update = updated(selection_outcome(vec![selected.into()]));
        assert_eq!(update.buffer, selected);
        assert_eq!(update.cursor, selected.chars().count());
        assert!(!update.placeholder);
    }

    #[test]
    fn empty_candidates_and_cancelled_selection_return_no_selection() {
        assert!(matches!(
            select(b"", "", "unchanged"),
            Ok(Outcome::NoSelection)
        ));
        assert!(matches!(
            selection_outcome(Vec::new()),
            Outcome::NoSelection
        ));
    }
}
