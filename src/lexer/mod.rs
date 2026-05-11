pub mod error;
pub mod lexer;
pub mod span;
pub mod token;

#[cfg(test)]
mod tests;

pub use error::{LexError, LexErrorKind};
pub use lexer::Lexer;
pub use span::Span;
pub use token::{Token, TokenKind};
