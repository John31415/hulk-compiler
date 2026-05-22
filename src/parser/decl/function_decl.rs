use crate::ast::{Decl, DeclKind, Expr, Spanned};
use crate::lexer::{Token, TokenKind};
use crate::parser::expr::block::block_parser;
use crate::parser::span_from_token_slice;
use chumsky::{error::Rich, prelude::*};

pub fn function_decl_parser<'src>(
    expr: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Decl, extra::Err<Rich<'src, Token>>> {
    let semi = select_ref! {
        Token {
            kind: TokenKind::Semi,
            ..
        } => ()
    };
    let ident = select_ref! {
        Token {
            kind: TokenKind::Identifier(name),
            ..
        } => name.clone()
    };
    let param = ident.then(
        select_ref! {
            Token {
                kind: TokenKind::Colon,
                ..
            } => ()
        }
        .ignore_then(ident.clone())
        .or_not(),
    );
    let params = param
        .separated_by(select_ref! {
            Token {
                kind: TokenKind::Comma,
                ..
            } => ()
        })
        .collect::<Vec<_>>()
        .delimited_by(
            select_ref! { Token { kind: TokenKind::LParen, .. } => () },
            select_ref! { Token { kind: TokenKind::RParen, .. } => () },
        );
    let signature = select_ref! {
        Token {
            kind: TokenKind::Function,
            ..
        } => ()
    }
    .ignore_then(ident)
    .then(params)
    .then(
        select_ref! {
            Token {
                kind: TokenKind::Colon,
                ..
            } => ()
        }
        .ignore_then(ident.clone())
        .or_not(),
    );
    let inline_body = select_ref! {
        Token {
            kind: TokenKind::Arrow,
            ..
        } => ()
    }
    .ignore_then(expr.clone());
    let block_body = block_parser(expr.clone());
    let body = choice((inline_body.then_ignore(semi), block_body));
    signature
        .then(body)
        .map_with(|(((name, params), return_type), body), span| {
            let span = span_from_token_slice(span.slice());
            Spanned::new(
                DeclKind::Function {
                    name,
                    params,
                    return_type,
                    body,
                },
                span,
            )
        })
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{decl::function_decl::*, expr::expr_parser, test_utils::tokenize};
    use chumsky::Parser;
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_decl_function_inline() {
        let source = "
        function pl(text, str: Number): String => print(text @@ str);
        ";

        let parser = function_decl_parser(expr_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_decl_function_block() {
        let source = "
        function p1() {
            1;
        }
        ";

        let parser = function_decl_parser(expr_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }
}
