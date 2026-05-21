use crate::ast::{Expr, ExprKind, Spanned};
use crate::lexer::{Span, Token, TokenKind};
use chumsky::{error::Rich, prelude::*};

pub fn assign_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    lower.clone()
        .then(
            select_ref! {
                Token {
                    kind: TokenKind::ColonEqual,
                    ..
                } => ()
            }
            .ignore_then(lower)
            .repeated()
            .collect::<Vec<_>>(),
        )
        .map(|(first, rest)| {
            rest.into_iter().rfold(first, |left, right| {
                let span = Span {
                    start: left.span.start,
                    end: right.span.end,
                };
                Spanned::new(
                    ExprKind::Assign {
                        target: Box::new(left),
                        value: Box::new(right),
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
        expr::{assign::*, primary::*},
        test_utils::tokenize,
    };
    use chumsky::Parser;
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_if_else() {
        let source = "
        a := 1
        ";

        let parser = assign_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }
}