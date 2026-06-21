use crate::ast::{BinaryOp, BinaryOpKind, Expr, ExprKind, Spanned};
use crate::lexer::{Token, TokenKind};
use crate::parser::span_from_token_slice;
use chumsky::{error::Rich, prelude::*, primitive::choice, recursive::Recursive};

pub fn exponent_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let exp_op = select_ref! {
        Token{kind: TokenKind::Caret, span, ..} => Spanned::new(BinaryOpKind::Pow, *span),
    };
    let mut exponent = Recursive::declare();
    let exponent_parser = lower
        .clone()
        .then(exp_op.then(exponent.clone()).or_not())
        .map(|(left, rest)| match rest {
            Some((op, right)) => binary_fold(left, (op, right)),
            None => left,
        });
    exponent.define(exponent_parser);
    exponent
}

pub fn product_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let mul_op = choice((
        select_ref! {Token { kind: TokenKind::Star, span, ..} => Spanned::new(BinaryOpKind::Mul, *span) },
        select_ref! {Token { kind: TokenKind::Slash, span, ..} => Spanned::new(BinaryOpKind::Div, *span) },
        select_ref! {Token { kind: TokenKind::Percent, span, ..} => Spanned::new(BinaryOpKind::Mod, *span) },
    ));
    lower
        .clone()
        .foldl(mul_op.then(lower.clone()).repeated(), binary_fold)
}

pub fn sum_at_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let add_op = choice((
        select_ref! {Token { kind: TokenKind::Plus, span, ..} => Spanned::new(BinaryOpKind::Add, *span) },
        select_ref! {Token { kind: TokenKind::Minus, span, ..} => Spanned::new(BinaryOpKind::Sub, *span) },
        select_ref! {Token { kind: TokenKind::At, span, ..} => Spanned::new(BinaryOpKind::Concat, *span) },
        select_ref! {Token { kind: TokenKind::AtAt, span, ..} => Spanned::new(BinaryOpKind::ConcatSpace, *span) },
    ));
    lower
        .clone()
        .foldl(add_op.then(lower.clone()).repeated(), binary_fold)
}

pub fn comparison_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let cmp_op = choice((
        select_ref! { Token { kind: TokenKind::Less, span, ..} => Spanned::new(BinaryOpKind::Less, *span) },
        select_ref! { Token { kind: TokenKind::Greater, span, ..} => Spanned::new(BinaryOpKind::Greater, *span) },
        select_ref! { Token { kind: TokenKind::LessEqual, span, ..} => Spanned::new(BinaryOpKind::LessEqual, *span) },
        select_ref! { Token { kind: TokenKind::GreaterEqual, span, ..} => Spanned::new(BinaryOpKind::GreaterEqual, *span) },
    ));
    lower
        .clone()
        .foldl(cmp_op.then(lower.clone()).repeated(), binary_fold)
}

pub fn is_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    lower
        .then(
            select_ref! {
                Token {
                    kind: TokenKind::Is,
                    ..
                } => ()
            }
            .ignore_then(select_ref! {
                Token {
                    kind: TokenKind::Identifier(name),
                    ..
                } => name.clone()
            })
            .or_not(),
        )
        .map_with(|(expr, type_name), span| {
            let span = span_from_token_slice(span.slice());
            match type_name {
                Some(type_name) => Spanned::new(
                    ExprKind::Is {
                        expr: Box::new(expr),
                        type_name,
                    },
                    span,
                ),
                None => expr,
            }
        })
}

pub fn as_expr_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    lower
        .then(
            select_ref! {
                Token {
                    kind: TokenKind::As,
                    ..
                } => ()
            }
            .ignore_then(select_ref! {
                Token {
                    kind: TokenKind::Identifier(name),
                    ..
                } => name.clone()
            })
            .or_not(),
        )
        .map_with(|(expr, type_name), span| {
            let span = span_from_token_slice(span.slice());
            match type_name {
                Some(type_name) => Spanned::new(
                    ExprKind::As {
                        expr: Box::new(expr),
                        type_name,
                    },
                    span,
                ),
                None => expr,
            }
        })
}

pub fn equality_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let eq_op = choice((
        select_ref! { Token { kind: TokenKind::DoubleEqual, span, ..} => Spanned::new(BinaryOpKind::DoubleEqual, *span) },
        select_ref! { Token { kind: TokenKind::NotEqual, span, ..} => Spanned::new(BinaryOpKind::NotEqual, *span) },
    ));
    lower
        .clone()
        .foldl(eq_op.then(lower.clone()).repeated(), binary_fold)
}

pub fn logical_and_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let and_op = select_ref! {
        Token {kind: TokenKind::Ampersand, span, ..} => Spanned::new(BinaryOpKind::And, *span),
    };
    lower
        .clone()
        .foldl(and_op.then(lower.clone()).repeated(), binary_fold)
}

pub fn logical_or_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let or_op = select_ref! {
        Token {kind: TokenKind::Pipe, span, ..} => Spanned::new(BinaryOpKind::Or, *span),
    };
    lower
        .clone()
        .foldl(or_op.then(lower.clone()).repeated(), binary_fold)
}

fn binary_fold(left: Expr, (op, right): (BinaryOp, Expr)) -> Expr {
    let span = left.span.union(&right.span);
    Spanned {
        node: ExprKind::Binary {
            left_expr: Box::new(left),
            op,
            right_expr: Box::new(right),
        },
        span,
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{
        expr::{binary::*, primary::*},
        test_utils::tokenize,
    };
    use chumsky::Parser;
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_exponent() {
        let source = "
        1^2^3
        ";

        let parser = exponent_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_product() {
        let source = "
        1*2/3
        ";

        let parser = product_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_sum() {
        let source = "
        1+2-3 @ \"hello\" @@ \" world\"
        ";

        let parser = sum_at_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_comparison() {
        let source = "
        1 >= 5
        ";

        let parser = comparison_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_is() {
        let source = "
        1 is Number
        ";

        let parser = is_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_as() {
        let source = "
        1 as Number
        ";

        let parser = as_expr_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_equality() {
        let source = "
        1 == 2 != 3
        ";

        let parser = equality_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_logical_and() {
        let source = "
        1 & 2
        ";

        let parser = logical_and_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_logical_or() {
        let source = "
        1 | 2
        ";

        let parser = logical_or_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }
}
