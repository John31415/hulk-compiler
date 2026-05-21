use crate::lexer::Lexer;

#[allow(dead_code)]
pub fn tokenize(src: &str) -> Vec<crate::lexer::Token> {
    Lexer::new(src).tokenize().expect("Lexical error")
}
