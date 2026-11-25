//! Command position and unfinished syntax checks for Zsh-tokenized input.

pub fn command_position(tokens: &[&str]) -> bool {
    let mut at_command = true;
    let mut wrapper = false;
    for token in tokens {
        if matches!(
            *token,
            "&&" | "||" | ";" | "|" | "|&" | "&" | "(" | "{" | "\n"
        ) {
            at_command = true;
            wrapper = false;
        } else if at_command {
            if matches!(*token, "sudo" | "env" | "doas" | "nohup" | "nice") {
                wrapper = true;
            } else if !matches!(
                *token,
                "then"
                    | "do"
                    | "else"
                    | "elif"
                    | "!"
                    | "time"
                    | "coproc"
                    | "nocorrect"
                    | "noglob"
                    | "exec"
                    | "builtin"
                    | "command"
            ) && !assignment(token)
                && !(wrapper && token.starts_with('-'))
            {
                at_command = false;
            }
        }
    }
    at_command
}

fn assignment(token: &str) -> bool {
    let Some((name, _)) = token.split_once('=') else {
        return false;
    };
    let mut chars = name.bytes();
    chars
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && chars.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Frame {
    Single,
    Ansi,
    Double,
    Backtick,
    Parens,
    Braces,
}

pub fn unfinished(input: &str) -> bool {
    use Frame::*;

    let mut stack = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        let frame = stack.last().copied();
        match (frame, ch) {
            (Some(Single | Ansi), '\'')
            | (Some(Double), '"')
            | (Some(Backtick), '\x60')
            | (Some(Parens), ')')
            | (Some(Braces), '}') => {
                stack.pop();
            }
            (Some(Single), _) => {}
            (_, '\\') => {
                chars.next();
            }
            (Some(Ansi), _) => {}
            (_, '$') => {
                let nested = match chars.peek() {
                    Some('(') => Parens,
                    Some('{') => Braces,
                    Some('\'') if frame != Some(Double) => Ansi,
                    _ => continue,
                };
                chars.next();
                stack.push(nested);
            }
            (_, '\x60') => stack.push(Backtick),
            (Some(Double), _) => {}
            (_, '\'') => stack.push(Single),
            (_, '"') => stack.push(Double),
            (Some(Parens), '(') => stack.push(Parens),
            _ => {}
        }
    }
    !stack.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_position_after_separators_modifiers_and_assignments() {
        assert!(command_position(&[
            "echo", "x", "&&", "FOO=1", "sudo", "-n", "noglob"
        ]));
        assert!(!command_position(&["echo", "then"]));
        assert!(!command_position(&["env", "echo"]));
    }

    #[test]
    fn recognizes_open_and_closed_nested_constructs() {
        for input in [
            "'gs",
            "\"gs",
            "$'gs",
            "$(echo gs",
            "$(\x60echo gs",
            "\x60echo gs",
            "${gs",
            "\"$(printf '%s' \"x\") gs",
        ] {
            assert!(unfinished(input), "{input:?}");
        }
        for input in [
            "echo '\"' && gs",
            "\"$(echo \"x\")\" && gs",
            "$'\\'' && gs",
            "${x:-${y}} gs",
        ] {
            assert!(!unfinished(input), "{input:?}");
        }
    }
}
