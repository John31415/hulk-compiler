#![allow(dead_code)]

use chumsky::Parser;

use crate::{ast::Program, lexer::Lexer, parser::program::program_parser};

pub fn parse_program(source: &str) -> Program {
    let tokens = Lexer::new(&source).tokenize().expect("Lexer error");
    let program = program_parser()
        .parse(&tokens.as_slice())
        .into_result()
        .expect("Parser error");
    program
}
