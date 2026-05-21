use crate::ast::{Expr, ExprKind, Spanned};
use crate::lexer::{Span, Token, TokenKind};
use chumsky::{error::Rich, prelude::*};

pub fn new_parser<'src>(
    expr: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    select_ref! {
        Token {
            kind: TokenKind::New,
            ..
        } => ()
    }
    .ignore_then(select_ref! {
        Token {
            kind: TokenKind::Identifier(name),
            ..
        } => name.clone()
    })
    .then(
        expr.clone()
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
            ),
    )
    .map_with(|(type_name, args), span| {
        let span = Span {
            start: span.span().start(),
            end: span.span().end(),
        };
        Spanned::new(ExprKind::New { type_name, args }, span)
    })
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{
        expr::{primary::primary_parser, new::new_parser},
        test_utils::tokenize,
    };
    use chumsky::{Parser, prelude::*};
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_new() {
        let source = "
        new Point(1, 3)
        ";

        let parser = new_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }
}
