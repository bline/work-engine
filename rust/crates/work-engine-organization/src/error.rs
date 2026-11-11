//! Typed errors for the codec and the scalar rules.

use std::fmt;

/// Where a codec error was found: a byte offset when parsing authored bytes, or a
/// JSON-pointer-style path when converting an in-memory value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Position {
    Offset(usize),
    Path(String),
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Position::Offset(offset) => write!(f, "byte {offset}"),
            Position::Path(path) if path.is_empty() => write!(f, "document root"),
            Position::Path(path) => write!(f, "path {path}"),
        }
    }
}

/// A refusal by the canonical codec. Ambiguous input is refused, never normalized.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CodecError {
    #[error("input is not valid UTF-8 (valid up to byte {valid_up_to})")]
    InvalidUtf8 { valid_up_to: usize },
    #[error("JSON syntax error at {at}: {detail}")]
    Syntax { at: Position, detail: &'static str },
    #[error("duplicate object key {key:?} at {at}")]
    DuplicateKey { key: String, at: Position },
    #[error(
        "non-integer number at {at}: only integers are accepted, floats and exponents are refused"
    )]
    NonIntegerNumber { at: Position },
    #[error("negative zero at {at}: `-0` is a non-canonical spelling of 0 and is refused")]
    NegativeZero { at: Position },
    #[error("integer out of range at {at}: allowed magnitude is at most 2^53 - 1")]
    IntegerOutOfRange { at: Position },
    #[error("explicit null at {at}: optional fields are absent or non-null")]
    ExplicitNull { at: Position },
    #[error("unpaired surrogate escape at {at}")]
    UnpairedSurrogate { at: Position },
    #[error("nesting deeper than {limit} levels at {at}")]
    TooDeep { limit: usize, at: Position },
    #[error("unknown codec declaration (domain {domain:?}, kind {kind:?}, version {version:?})")]
    UnknownCodec {
        domain: String,
        kind: String,
        version: String,
    },
}

/// A refusal by the identifier syntax `^[a-z0-9][a-z0-9._:-]{0,127}$`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    #[error("identifier is empty")]
    Empty,
    #[error("identifier has {len} bytes; the limit is 128")]
    TooLong { len: usize },
    #[error(
        "identifier starts with {found:?}; it must start with a lowercase ASCII letter or digit"
    )]
    InvalidStart { found: char },
    #[error(
        "identifier has {found:?} at byte {index}; allowed are lowercase ASCII letters, digits and . _ : -"
    )]
    InvalidCharacter { index: usize, found: char },
}
