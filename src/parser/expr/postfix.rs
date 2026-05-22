use crate::ast::{Expr, ExprKind, Spanned};
use crate::lexer::{Span, Token, TokenKind};
use crate::parser::span_from_token_slice;
use chumsky::{error::Rich, prelude::*, primitive::choice};

enum PostfixOp {
    Call(Vec<Expr>, Span),
    Property(String, Span),
    MethodCall(String, Vec<Expr>, Span),
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
    .map_with(|((_, method), args), e| {
        PostfixOp::MethodCall(method, args, span_from_token_slice(e.slice()))
    });
    let property_access = select_ref! {
        Token { kind: TokenKind::Dot, .. } => ()
    }
    .then(identifier.clone())
    .map_with(|(_, property), e| PostfixOp::Property(property, span_from_token_slice(e.slice())));
    let call = args
        .clone()
        .map_with(|args, e| PostfixOp::Call(args, span_from_token_slice(e.slice())));
    let postfix_op = choice((method_call, property_access, call));
    lower
        .clone()
        .then(postfix_op.repeated().collect::<Vec<_>>())
        .map(|(expr, ops)| {
            ops.into_iter().fold(expr, |expr, op| match op {
                PostfixOp::Call(args, op_span) => match expr.node {
                    ExprKind::Variable(name) => {
                        let span = expr.span.union(&op_span);
                        Spanned::new(ExprKind::Call { name, args }, span)
                    }
                    _ => {
                        panic!("Invalid function call target")
                    }
                },
                PostfixOp::Property(property, op_span) => {
                    let span = expr.span.union(&op_span);
                    Spanned::new(
                        ExprKind::PropertyAccess {
                            obj: Box::new(expr),
                            property,
                        },
                        span,
                    )
                }
                PostfixOp::MethodCall(method, args, op_span) => {
                    let span = expr.span.union(&op_span);
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
