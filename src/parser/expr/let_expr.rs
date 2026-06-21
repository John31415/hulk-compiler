use crate::ast::{Expr, ExprKind, Spanned};
use crate::lexer::{Span, Token, TokenKind};
use chumsky::{error::Rich, prelude::*};

pub fn let_expr_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let binding = select_ref! {
        Token {
            kind: TokenKind::Identifier(name),
            ..
        } => name.clone()
    }
    .then(
        select_ref! {
            Token {
                kind: TokenKind::Colon,
                ..
            } => ()
        }
        .ignore_then(
            select_ref! {
                Token {
                    kind: TokenKind::Identifier(name),
                    ..
                } => name.clone()
            }
        )
        .or_not()
    )
    .then_ignore(select_ref! {
        Token {
            kind: TokenKind::Equal,
            .. } => ()
    })
    .then(lower.clone());
    let bindings = binding
        .separated_by(select_ref! {
            Token {
                kind: TokenKind::Comma,
                ..
            } => ()
        })
        .at_least(1)
        .collect::<Vec<_>>();
    select_ref! {
        Token {
            kind: TokenKind::Let,
            ..
        } => ()
    }
    .ignore_then(bindings)
    .then_ignore(select_ref! {
        Token {
            kind: TokenKind::In,
            ..
        } => ()
    })
    .then(lower.clone())
    .map_with(|(bindings, body), _| {
        bindings
            .into_iter()
            .rev()
            .fold(body, |body, ((name, type_name), value)| {
                let span = Span {
                    start: value.span.start,
                    end: body.span.end,
                };
                Spanned::new(
                    ExprKind::Let {
                        name,
                        type_name,
                        value: Box::new(value),
                        body: Box::new(body),
                    },
                    span,
                )
            })
    })
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{
        expr::{let_expr::*, primary::*},
        test_utils::tokenize,
    };
    use chumsky::Parser;
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_exponent() {
        let source = "
        let a = 1 in 2
        ";

        let parser = let_expr_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }
}