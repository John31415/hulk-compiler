use crate::ast::{Decl, DeclKind, Expr, InheritInfoKind, Spanned, TypeFeaturesKind};
use crate::lexer::{Span, Token, TokenKind};
use crate::parser::expr::block::block_parser;
use chumsky::{error::Rich, prelude::*};

pub fn type_decl_parser<'src>(
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
    let opt_type = select_ref! {
        Token {
            kind: TokenKind::Colon,
            ..
        } => ()
    }
    .ignore_then(ident.clone())
    .or_not();
    let parent_args = expr
        .clone()
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
    let inherited = select_ref! {
        Token {
            kind: TokenKind::Inherits,
            ..
        } => ()
    }
    .ignore_then(ident.clone())
    .then(parent_args.clone().or_not())
    .map_with(|(parent_name, args), span| {
        let span = Span {
            start: span.span().start(),
            end: span.span().end(),
        };
        Spanned::new(InheritInfoKind { parent_name, args }, span)
    });
    let params = ident
        .clone()
        .then(opt_type.clone())
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
            kind: TokenKind::Type,
            ..
        } => ()
    }
    .ignore_then(ident.clone())
    .then(params.clone().or_not())
    .then(inherited.or_not());
    let attributes = ident
        .clone()
        .then(opt_type.clone())
        .then_ignore(select_ref! {
            Token {
                kind: TokenKind::Equal,
                ..
            } => ()
        })
        .then(expr.clone())
        .then_ignore(semi.clone())
        .map(|((name, type_name), default)| {
            let span = Span {
                start: default.span.start,
                end: default.span.end,
            };
            Spanned::new(
                TypeFeaturesKind::Attribute {
                    name,
                    type_name,
                    default: Some(default),
                },
                span,
            )
        });
    let inline_body = select_ref! {
        Token {
            kind: TokenKind::Arrow,
            ..
        } => ()
    }
    .ignore_then(expr.clone());
    let block_body = block_parser(expr.clone());
    let body = choice((inline_body.then_ignore(semi.clone()), block_body));
    let methods = ident
        .clone()
        .then(params.clone())
        .then(opt_type.clone())
        .then(body)
        .map(|(((name, params), return_type), body)| {
            let span = Span {
                start: body.span.start,
                end: body.span.end,
            };
            Spanned::new(
                TypeFeaturesKind::Method {
                    name,
                    params,
                    return_type,
                    body,
                },
                span,
            )
        });
    let features = attributes
        .or(methods)
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(
            select_ref! { Token {kind: TokenKind::LBrace, .. } => () },
            select_ref! { Token {kind: TokenKind::RBrace, .. } => () },
        );
    signature
        .then(features)
        .map_with(|(((name, params), parent), features), span| {
            let span = Span {
                start: span.span().start(),
                end: span.span().end(),
            };
            Spanned::new(
                DeclKind::Type {
                    name,
                    params,
                    parent,
                    features,
                },
                span,
            )
        })
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{decl::type_decl::*, expr::expr_parser, test_utils::tokenize};
    use chumsky::Parser;
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_decl_type_basic() {
        let source = "
        type Point {
            x = 0;
            y = 0;

            getX() => self.x;
            getY() => self.y;

            setX(x) => self.x := x;
            setY(y) => self.y := y;
        }
        ";

        let parser = type_decl_parser(expr_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_decl_type_inherits() {
        let source = "
        type Knight inherits Person {
            name() {
                \"Sir\" @@ base();
            }
        }
        ";

        let parser = type_decl_parser(expr_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_decl_type_inherits_params() {
        let source = "
        type PolarPoint(phi: Number, rho) inherits Point(rho * sin(phi), rho * cos(phi)) {
            rho(): Number => sqrt(self.getX() ^ 2 + self.getY() ^ 2);
        }
        ";

        let parser = type_decl_parser(expr_parser().boxed());

        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }
}
