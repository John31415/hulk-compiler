use super::Span;
use logos::{Lexer, Logos};

#[allow(dead_code)]
#[derive(Logos, Debug, Clone, PartialEq)]
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
    #[regex(r"(0|[1-9][0-9]*)(\.[0-9]+)?", |lex| lex.slice().parse().ok())]
    LiteralNumber(f64),
    #[regex(r#""([^"\\]|\\.)*""#, process_string)]
    LiteralString(String),
    #[token("true")]
    LiteralTrue,
    #[token("false")]
    LiteralFalse,

    // End of file
    EOF,
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[allow(dead_code)]
fn process_string(lex: &mut Lexer<TokenKind>) -> String {
    let s = lex.slice();
    let unquoted = &s[1..s.len() - 1];
    let mut result = String::with_capacity(unquoted.len());
    let mut chars = unquoted.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    result
}