use std::fmt::{Display, Formatter};
use std::num::ParseIntError;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseHashError {
    /// The error returned when the numeric hash string doesn't begin with "0x"
    MissingPrefix,
    /// The error returned when the hexadecimal part of the hash string cannot be parsed
    ParseError(ParseIntError),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FromLabelError {
    /// The error returned only when the static label map is bidirectional, and a label
    /// cannot be matched to a hash
    LabelNotFound(String),
    /// The error returned when the hexadecimal part of the hash string cannot be parsed
    ParseError(ParseIntError),
}

impl From<ParseIntError> for ParseHashError {
    fn from(err: ParseIntError) -> Self {
        Self::ParseError(err)
    }
}

impl From<ParseIntError> for FromLabelError {
    fn from(err: ParseIntError) -> Self {
        Self::ParseError(err)
    }
}

impl Display for FromLabelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
