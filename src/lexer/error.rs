use super::Span;
use crate::diagnostics::{Diagnostic, Label};

#[derive(Debug, Clone, PartialEq)]
pub enum LexErrorKind {
    InvalidEscapeSequence,
    LeadingZero,
    MalformedNumber,
    NumericOverflow,
    UnexpectedCharacter,
    UnclosedString,
}

impl Default for LexErrorKind {
    fn default() -> Self {
        Self::UnexpectedCharacter
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: Span,
}

impl LexError {
    pub fn new(kind: LexErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl From<LexError> for Diagnostic {
    fn from(value: LexError) -> Self {
        let message = match value.kind {
            LexErrorKind::InvalidEscapeSequence => "invalid escape sequence",
            LexErrorKind::LeadingZero => "leading zeros",
            LexErrorKind::MalformedNumber => "malformed number",
            LexErrorKind::NumericOverflow => "numeric overflow",
            LexErrorKind::UnexpectedCharacter => "unexpected character",
            LexErrorKind::UnclosedString => "unclosed string literal",
        };
        Diagnostic::error(message, value.span).with_label(Label::new(message, value.span))
    }
}
