use crate::ast::{Expr, ExprKind, Spanned, UnaryOpKind};
use crate::lexer::{Span, Token, TokenKind};
use chumsky::{
    error::Rich,
    prelude::*,
    primitive::choice,
    recursive::{Indirect, Recursive},
};

pub fn unary_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let unary_op = choice((
        select_ref! {Token { kind: TokenKind::Minus, span, ..} => Spanned::new(UnaryOpKind::Neg, *span)},
        select_ref! {Token { kind: TokenKind::Bang, span, ..} => Spanned::new(UnaryOpKind::Not, *span)},
    ));
    let mut unary: Recursive<
        Indirect<'src, 'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>>,
    > = Recursive::declare();
    unary.define(
        unary_op
            .then(unary.clone())
            .map(|(op, expr)| {
                let span = Span {
                    start: op.span.start,
                    end: expr.span.end,
                };
                Spanned::new(
                    ExprKind::Unary {
                        op,
                        expr: Box::new(expr),
                    },
                    span,
                )
            })
            .or(lower),
    );
    unary
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{
        expr::{primary::primary_parser, unary::unary_parser},
        test_utils::tokenize,
    };
    use chumsky::{Parser, prelude::*};
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_unary_neg() {
        let source = "
        -42
        ";

        let parser = unary_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_unary_not() {
        let source = "
        !false
        ";

        let parser = unary_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }
}
