pub mod decl;
pub mod error;
pub mod expr;
pub mod program;
pub mod test_utils;

use crate::lexer::{Span, Token};

pub(crate) fn span_from_token_slice(tokens: &[Token]) -> Span {
    match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => Span::new(first.span.start, last.span.end),
        _ => Span::new(0, 0),
    }
}

#[cfg(test)]
mod tests;
