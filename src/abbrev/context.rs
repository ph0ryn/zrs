//! Command position and unfinished syntax checks for Zsh-tokenized input.

#[derive(Clone, Copy, PartialEq, Eq)]
enum Wrapper {
    Sudo,
    Env,
    Doas,
    Nohup,
    Nice,
    Command,
}

impl Wrapper {
    fn parse(token: &str) -> Option<Self> {
        match token {
            "sudo" => Some(Self::Sudo),
            "env" => Some(Self::Env),
            "doas" => Some(Self::Doas),
            "nohup" => Some(Self::Nohup),
            "nice" => Some(Self::Nice),
            "command" => Some(Self::Command),
            _ => None,
        }
    }

    fn option_takes_value(self, token: &str) -> bool {
        match self {
            Self::Sudo => matches!(
                token,
                "-u" | "-g"
                    | "-C"
                    | "-D"
                    | "-h"
                    | "-p"
                    | "-r"
                    | "-t"
                    | "-T"
                    | "-U"
                    | "--user"
                    | "--group"
                    | "--close-from"
                    | "--chdir"
                    | "--host"
                    | "--prompt"
                    | "--role"
                    | "--type"
                    | "--command-timeout"
                    | "--other-user"
            ),
            Self::Env => matches!(
                token,
                "-u" | "-C" | "-S" | "--unset" | "--chdir" | "--split-string"
            ),
            Self::Doas => matches!(token, "-u" | "-C"),
            Self::Nice => matches!(token, "-n" | "--adjustment"),
            Self::Nohup | Self::Command => false,
        }
    }
}

fn is_modifier(token: &str) -> bool {
    matches!(
        token,
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
    )
}

pub fn command_position(tokens: &[&str]) -> bool {
    let mut at_command = true;
    let mut wrapper: Option<Wrapper> = None;
    let mut expect_option_value = false;
    for token in tokens {
        if matches!(
            *token,
            "&&" | "||" | ";" | "|" | "|&" | "&" | "(" | "{" | "\n"
        ) {
            at_command = true;
            wrapper = None;
            expect_option_value = false;
        } else if at_command {
            if expect_option_value {
                expect_option_value = false;
            } else if let Some(current) = wrapper {
                if *token == "--" {
                    wrapper = None;
                } else if token.starts_with('-') {
                    expect_option_value = current.option_takes_value(token);
                } else if assignment(token) {
                    // Environment assignments after wrappers (e.g. `sudo FOO=1 cmd`) stay at command position.
                } else if let Some(next) = Wrapper::parse(token) {
                    wrapper = Some(next);
                } else if is_modifier(token) {
                    wrapper = None;
                } else {
                    at_command = false;
                    wrapper = None;
                }
            } else if let Some(next) = Wrapper::parse(token) {
                wrapper = Some(next);
            } else if !is_modifier(token) && !assignment(token) {
                at_command = false;
            }
        }
    }
    at_command && !expect_option_value
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
            (Some(Braces), '{') => stack.push(Braces),
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
        assert!(command_position(&["sudo", "-u", "root"]));
        assert!(command_position(&["sudo", "-u", "root", "--"]));
        assert!(command_position(&["env", "-u", "FOO", "BAR=1"]));
        assert!(command_position(&["nice", "-n", "10", "sudo", "-E"]));
        assert!(command_position(&["command", "-p"]));
        assert!(!command_position(&["sudo", "-u"]));
        assert!(!command_position(&["nice", "-n"]));
        assert!(!command_position(&["sudo", "--", "-n"]));
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
            "${x:-{a} gs",
            "\"$(printf '%s' \"x\") gs",
        ] {
            assert!(unfinished(input), "{input:?}");
        }
        for input in [
            "echo '\"' && gs",
            "\"$(echo \"x\")\" && gs",
            "$'\\'' && gs",
            "${x:-${y}} gs",
            "${x:-{a,b}} && gs",
        ] {
            assert!(!unfinished(input), "{input:?}");
        }
    }
}
