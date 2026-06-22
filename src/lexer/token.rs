use crate::lexer::LexErrorKind;

use super::Span;
use logos::{Lexer, Logos};

#[allow(dead_code)]
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(error = LexErrorKind)]
pub enum TokenKind {
    // Spaces
    #[regex(r"[ \t\n\f]+", logos::skip)]
    // Comments
    #[regex(r"//.*", logos::skip)]
    // Invalid
    Error,

    // Keywords
    #[token("let")]
    Let,
    #[token("in")]
    In,
    #[token("function")]
    Function,
    #[token("if")]
    If,
    #[token("elif")]
    Elif,
    #[token("else")]
    Else,
    #[token("for")]
    For,
    #[token("while")]
    While,
    #[token("type")]
    Type,
    #[token("inherits")]
    Inherits,
    #[token("new")]
    New,
    #[token("is")]
    Is,
    #[token("as")]
    As,
    #[token("interface")]
    Interface,
    #[token("protocol")]
    Protocol,
    #[token("extends")]
    Extends,

    // Operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("^")]
    Caret,
    #[token("@")]
    At,
    #[token("@@")]
    AtAt,
    #[token("=")]
    Equal,
    #[token(":=")]
    ColonEqual,
    #[token("<")]
    Less,
    #[token(">")]
    Greater,
    #[token("<=")]
    LessEqual,
    #[token(">=")]
    GreaterEqual,
    #[token("==")]
    DoubleEqual,
    #[token("!=")]
    NotEqual,
    #[token("&")]
    Ampersand,
    #[token("|")]
    Pipe,
    #[token("!")]
    Bang,

    // Punctuation
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token(";")]
    Semi,
    #[token(",")]
    Comma,
    #[token("=>")]
    Arrow,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,

    // Literals
    #[regex(r"[a-zA-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),
    #[regex(r"[0-9]+[\.0-9]*", validate_process_number)]
    LiteralNumber(f64),
    #[regex(r#""([^"\\]|\\.)*(")?"#, validate_process_string)]
    LiteralString(String),
    #[token("true")]
    LiteralTrue,
    #[token("false")]
    LiteralFalse,

    // End of file
    EOF,
}

fn validate_process_number(lex: &mut Lexer<TokenKind>) -> Result<f64, LexErrorKind> {
    let slice = lex.slice();
    if slice.starts_with('0') && slice.len() > 1 && !slice.starts_with("0.") {
        return Err(LexErrorKind::LeadingZero);
    }
    let dot_count = slice.matches('.').count();
    if dot_count > 1 || slice.ends_with('.') {
        return Err(LexErrorKind::MalformedNumber);
    }
    match slice.parse::<f64>() {
        Ok(val) if val.is_infinite() => Err(LexErrorKind::NumericOverflow),
        Ok(val) => Ok(val),
        Err(_) => Err(LexErrorKind::MalformedNumber),
    }
}

fn validate_process_string(lex: &mut Lexer<TokenKind>) -> Result<String, LexErrorKind> {
    let slice = lex.slice();
    if !slice.ends_with('"') || slice.len() < 2 {
        return Err(LexErrorKind::UnclosedString);
    }
    let unquoted = &slice[1..slice.len() - 1];
    let mut result = String::with_capacity(unquoted.len());
    let mut chars = unquoted.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some(_invalid_char) => return Err(LexErrorKind::InvalidEscapeSequence),
                None => return Err(LexErrorKind::UnclosedString),
            }
        } else {
            result.push(c);
        }
    }
    Ok(result)
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
