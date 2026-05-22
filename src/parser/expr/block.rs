use crate::ast::{Expr, ExprKind, Spanned};
use crate::lexer::{Token, TokenKind};
use crate::parser::span_from_token_slice;
use chumsky::{error::Rich, prelude::*};

pub fn block_parser<'src>(
    expr: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let semi = select_ref! {
        Token {
            kind: TokenKind::Semi,
            ..
        } => ()
    };
    expr.clone()
        .then_ignore(semi.clone())
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(
            select_ref! { Token { kind: TokenKind::LBrace, .. } => () },
            select_ref! { Token { kind: TokenKind::RBrace, .. } => () },
        )
        .map_with(|exprs, e| {
            Spanned::new(ExprKind::Block(exprs), span_from_token_slice(e.slice()))
        })
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{
        expr::{block::block_parser, primary::primary_parser},
        test_utils::tokenize,
    };
    use chumsky::{Parser, prelude::*};
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_block_basic() {
        let source = "
        {a; b; c;}
        ";

        let parser = block_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_block_semicolon() {
        let source = "
        {a; b; c;}
        ";

        let parser = block_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_block_semicolon_fail() {
        let source = "
        {a; b c;}
        ";

        let parser = block_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        assert!(parser.check(&tokens).into_result().is_err())
    }

    #[test]
    fn parser_block_brace_fail() {
        let source = "
        {a; b; c;
        ";

        let parser = block_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        assert!(parser.check(&tokens).into_result().is_err())
    }
}
