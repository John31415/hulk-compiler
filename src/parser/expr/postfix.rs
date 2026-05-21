use crate::ast::{Expr, ExprKind, Spanned};
use crate::lexer::{Span, Token, TokenKind};
use chumsky::{error::Rich, prelude::*, primitive::choice};

enum PostfixOp {
    Call(Vec<Expr>),
    Property(String),
    MethodCall(String, Vec<Expr>),
}

pub fn postfix_parser<'src>(
    expr: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let args = expr
        .clone()
        .separated_by(select_ref! {
            Token { kind: TokenKind::Comma, ..} => ()
        })
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(
            select_ref! {
                Token { kind: TokenKind::LParen, .. } => ()
            },
            select_ref! {
                Token { kind: TokenKind::RParen, .. } => ()
            },
        );
    let identifier = select_ref! {
        Token { kind: TokenKind::Identifier(name), .. } => name.clone()
    };
    let method_call = select_ref! {
        Token { kind: TokenKind::Dot, .. } => ()
    }
    .then(identifier.clone())
    .then(args.clone())
    .map(|((_, method), args)| PostfixOp::MethodCall(method, args));
    let property_access = select_ref! {
        Token { kind: TokenKind::Dot, .. } => ()
    }
    .then(identifier.clone())
    .map(|(_, property)| PostfixOp::Property(property));
    let call = args.clone().map(PostfixOp::Call);
    let postfix_op = choice((method_call, property_access, call));
    lower
        .clone()
        .then(postfix_op.repeated().collect::<Vec<_>>())
        .map(|(expr, ops)| {
            ops.into_iter().fold(expr, |expr, op| match op {
                PostfixOp::Call(args) => match expr.node {
                    ExprKind::Variable(name) => {
                        let span = Span {
                            start: expr.span.start,
                            end: expr.span.end,
                        };
                        Spanned::new(ExprKind::Call { name, args }, span)
                    }
                    _ => {
                        panic!("Invalid function call target")
                    }
                },
                PostfixOp::Property(property) => {
                    let span = Span {
                        start: expr.span.start,
                        end: expr.span.end,
                    };
                    Spanned::new(
                        ExprKind::PropertyAccess {
                            obj: Box::new(expr),
                            property,
                        },
                        span,
                    )
                }
                PostfixOp::MethodCall(method, args) => {
                    let span = Span {
                        start: expr.span.start,
                        end: expr.span.end,
                    };
                    Spanned::new(
                        ExprKind::MethodCall {
                            obj: Box::new(expr),
                            method,
                            args,
                        },
                        span,
                    )
                }
            })
        })
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{
        expr::{postfix::postfix_parser, primary::primary_parser},
        test_utils::tokenize,
    };
    use chumsky::{Parser, prelude::*};
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_postfix_call() {
        let source = "
        print(\"Hello World\")
        ";

        let parser = postfix_parser(primary_parser().boxed(), primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_postfix_property() {
        let source = "
        point.x
        ";

        let parser = postfix_parser(primary_parser().boxed(), primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_postfix_method_call() {
        let source = "
        vector.magnitude()
        ";

        let parser = postfix_parser(primary_parser().boxed(), primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_postfix_all() {
        let source = "
        tan(p).origin.translate(1, 2)
        ";

        let parser = postfix_parser(primary_parser().boxed(), primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }
}
