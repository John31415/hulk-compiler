use crate::ast::{Program, ProgramKind, Spanned};
use crate::lexer::{Token, TokenKind};
use crate::parser::decl::decl_parser;
use crate::parser::expr::block::block_parser;
use crate::parser::span_from_token_slice;
use crate::parser::expr::expr_parser;
use chumsky::{error::Rich, prelude::*};

pub fn program_parser<'src>()
-> impl Parser<'src, &'src [Token], Program, extra::Err<Rich<'src, Token>>> {
    let semi = select_ref! {
        Token {
            kind: TokenKind::Semi,
            ..
        } => ()
    };
    let expr = expr_parser().boxed();
    let block = block_parser(expr.clone());
    decl_parser(expr.clone())
        .repeated()
        .collect::<Vec<_>>()
        .or_not()
        .then(choice((
            block.then_ignore(semi.or_not()),
            expr.clone().then_ignore(semi),
        )))
        .then_ignore(select_ref! {
            Token {
                kind: TokenKind::EOF,
                ..
            } => ()
        })
        .map_with(|(decls, body), span| {
            let span = span_from_token_slice(span.slice());
            Spanned::new(ProgramKind { delcs: decls, body }, span)
        })
}

#[cfg(test)]
mod tests {
    use crate::parser::{program::*, test_utils::tokenize};
    use chumsky::Parser;
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_program_basic() {
        let source = "
        42;
        ";

        let tokens = tokenize(source);

        let parser = program_parser();

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_program_basic_entry_point() {
        let source = "
        type Point { x = 0; }

        function f() { 1; }

        42;
        ";

        let tokens = tokenize(source);

        let parser = program_parser();

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_program_block_entry_point() {
        let source = "
        type Point { x = 0; }

        function f() { 1; }

        {
            42;
        }
        ";

        let tokens = tokenize(source);

        let parser = program_parser();

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_program_block_entry_point_semi() {
        let source = "
        type Point { x = 0; }

        function f() { 1; }

        {
            42;
        };
        ";

        let tokens = tokenize(source);

        let parser = program_parser();

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }
}
