use super::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum LexErrorKind {
    UnexpectedCharacter,
    UnclosedString,
    Unknown,
}

impl LexErrorKind {
    pub fn from_slice(slice: &str) -> Self {
        if slice.is_empty() {
            return Self::Unknown;
        }
        match slice {
            s if s.starts_with('"') && (s.len() == 1 || !s.ends_with('"')) => Self::UnclosedString,
            _ => Self::UnexpectedCharacter
        }
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
