use logos::Logos;

use crate::lexer::{LexError, Span, Token, TokenKind};

pub struct Lexer<'a> {
    inner: logos::Lexer<'a, TokenKind>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            inner: TokenKind::lexer(input),
        }
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        match self.inner.next() {
            Some(Ok(kind)) => {
                let span = Span::from_range(self.inner.span());
                Ok(Token { kind, span })
            }
            Some(Err(error_kind)) => {
                let span = Span::from_range(self.inner.span());
                Err(LexError::new(error_kind, span))
            }
            None => {
                let span = Span::from_range(self.inner.span());
                Ok(Token {
                    kind: TokenKind::EOF,
                    span,
                })
            }
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, Vec<LexError>> {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        loop {
            match self.next_token() {
                Ok(token) => {
                    let is_eof = token.kind == TokenKind::EOF;
                    tokens.push(token);
                    if is_eof {
                        break;
                    }
                }
                Err(error) => errors.push(error),
            }
        }
        if errors.is_empty() {
            Ok(tokens)
        } else {
            Err(errors)
        }
    }
}
