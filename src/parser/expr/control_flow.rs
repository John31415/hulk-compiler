use crate::ast::{Expr, ExprKind, Spanned};
use crate::lexer::{Span, Token, TokenKind};
use chumsky::{error::Rich, prelude::*};

pub fn if_expr_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let mut if_chain = Recursive::declare();
    let else_branch = select_ref! {
        Token {
            kind: TokenKind::Else,
            ..
        } => ()
    }
    .ignore_then(lower.clone());
    let elif_branch = select_ref! {
        Token {
            kind: TokenKind::Elif,
            ..
        } => ()
    }
    .ignore_then(if_chain.clone());
    let tail = elif_branch.or(else_branch);
    let condition = lower.clone().delimited_by(
        select_ref! { Token { kind: TokenKind::LParen, .. } => () },
        select_ref! { Token { kind: TokenKind::RParen, .. } => () },
    );
    let chain_parser =
        condition
            .then(lower.clone())
            .then(tail)
            .map(|((condition, then_branch), else_branch)| {
                let end = else_branch.span.end;
                let span = Span {
                    start: condition.span.start,
                    end,
                };
                Spanned::new(
                    ExprKind::If {
                        condition: Box::new(condition),
                        then_branch: Box::new(then_branch),
                        else_branch: Some(Box::new(else_branch)),
                    },
                    span,
                )
            });
    if_chain.define(chain_parser);
    select_ref! {
        Token {
            kind: TokenKind::If,
            ..
        } => ()
    }
    .ignore_then(if_chain)
}

pub fn while_expr_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let condition = lower.clone().delimited_by(
        select_ref! { Token { kind: TokenKind::LParen, .. } => () },
        select_ref! { Token { kind: TokenKind::RParen, .. } => () },
    );
    select_ref! {
        Token {
            kind: TokenKind::While,
            ..
        } => ()
    }
    .ignore_then(condition)
    .then(lower.clone())
    .map(|(condition, body)| {
        let span = Span {
            start: condition.span.start,
            end: body.span.end,
        };
        Spanned::new(
            ExprKind::While {
                condition: Box::new(condition),
                body: Box::new(body),
            },
            span,
        )
    })
}

pub fn for_expr_parser<'src>(
    lower: impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let variable = select_ref! {
        Token {
            kind: TokenKind::Identifier(name),
            ..
        } => name.clone()
    };
    let header = variable
        .then_ignore(select_ref! {
            Token {
                kind: TokenKind::In,
                ..
            } => ()
        })
        .then(lower.clone())
        .delimited_by(
            select_ref! { Token { kind: TokenKind::LParen, .. } => () },
            select_ref! { Token { kind: TokenKind::RParen, .. } => () },
        );
    select_ref! {
        Token {
            kind: TokenKind::For,
            ..
        }
    }
    .ignore_then(header)
    .then(lower.clone())
    .map(|((var, iterable), body)| {
        let span = Span {
            start: iterable.span.start,
            end: body.span.end,
        };
        Spanned::new(
            ExprKind::For {
                var,
                iterable: Box::new(iterable),
                body: Box::new(body),
            },
            span,
        )
    })
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{
        expr::{control_flow::*, primary::*},
        test_utils::tokenize,
    };
    use chumsky::Parser;
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_if_else() {
        let source = "
        if (true) 1
        else 2
        ";

        let parser = if_expr_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_if_elif_else() {
        let source = "
        if (true) 1
        elif (false) 2
        else 3
        ";

        let parser = if_expr_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_while() {
        let source = "
        while (true) 1
        ";

        let parser = while_expr_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_for() {
        let source = "
        for (i in L) 1
        ";

        let parser = for_expr_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_if_fail() {
        let source = "
        if (true) 1
        ";

        let parser = if_expr_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        assert!(parser.check(&tokens).into_result().is_err())
    }

    #[test]
    fn parser_snapshot_if_elif_else_fail() {
        let source = "
        if (true) 1
        elif 2
        else 3
        ";

        let parser = if_expr_parser(primary_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        assert!(parser.check(&tokens).into_result().is_err())
    }
}
