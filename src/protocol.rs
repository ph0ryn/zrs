//! NUL-delimited protocol between ZLE and the CLI.

use std::io::Write;

use crate::error::ZrsError;

#[derive(Debug, PartialEq, Eq)]
pub struct Update {
    pub buffer: String,
    pub cursor: usize,
    pub placeholder: bool,
}

impl Update {
    pub fn new(left: &str, right: &str, placeholder: bool) -> Self {
        Self {
            buffer: format!("{left}{right}"),
            cursor: left.chars().count(),
            placeholder,
        }
    }
}

pub enum Outcome {
    Updated(Update),
    NoMatch,
    NoSelection,
}

impl From<Option<Update>> for Outcome {
    fn from(update: Option<Update>) -> Self {
        update.map_or(Self::NoMatch, Self::Updated)
    }
}

impl Outcome {
    pub fn write(self, mut out: impl Write) -> Result<u8, ZrsError> {
        match self {
            Self::Updated(update) => {
                if update.buffer.contains('\0') {
                    return Err(ZrsError::Protocol("buffer contains a NUL byte"));
                }
                let kind = if update.placeholder {
                    "placeholder"
                } else {
                    "normal"
                };
                write!(out, "{kind}\0{}\0{}\0", update.buffer, update.cursor)?;
                Ok(0)
            }
            Self::NoMatch => Ok(2),
            Self::NoSelection => Ok(3),
        }
    }
}
