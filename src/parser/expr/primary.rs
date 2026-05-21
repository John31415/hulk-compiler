use crate::ast::{Expr, ExprKind, LiteralKind, Spanned};
use crate::lexer::{Token, TokenKind};
use chumsky::{error::Rich, prelude::*, primitive::choice};

pub fn primary_parser<'src>()
-> impl Parser<'src, &'src [Token], Expr, extra::Err<Rich<'src, Token>>> {
    let variable = select_ref! {
        Token {kind: TokenKind::Identifier(name), span, ..} =>
            Spanned {
                node: ExprKind::Variable(name.clone()),
                span: *span,
            }
    };
    let literal = choice((
        select_ref! {
            Token {kind: TokenKind::LiteralNumber(value), span, ..} =>
                Spanned {
                    node: ExprKind::Literal(Spanned::new(LiteralKind::Number(*value), *span)),
                    span: *span,
                }
        },
        select_ref! {
            Token {kind: TokenKind::LiteralString(text), span, ..} =>
                Spanned {
                    node: ExprKind::Literal(Spanned::new(LiteralKind::String(text.clone()), *span)),
                    span: *span,
                }
        },
        select_ref! {
            Token {kind: TokenKind::LiteralTrue, span, ..} =>
                Spanned {
                    node: ExprKind::Literal(Spanned::new(LiteralKind::Bool(true), *span)),
                    span: *span,
                }
        },
        select_ref! {
            Token {kind: TokenKind::LiteralFalse, span, ..} =>
                Spanned {
                    node: ExprKind::Literal(Spanned::new(LiteralKind::Bool(false), *span)),
                    span: *span,
                }
        },
    ));
    choice((variable, literal))
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Token, TokenKind};
    use crate::parser::{expr::primary::primary_parser, test_utils::tokenize};
    use chumsky::{Parser, prelude::*};
    use insta::assert_yaml_snapshot;

    #[test]
    fn parser_snapshot_primary_literal() {
        let source = "
        42.42
        ";

        let parser = primary_parser();
        
        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });

        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }

    #[test]
    fn parser_snapshot_primary_variable() {
        let source = "
        var
        ";
        
        let parser = primary_parser();
        
        let tokens = tokenize(source);

        let parser = parser.then_ignore(select_ref! { Token { kind: TokenKind::EOF, .. } => () });
        
        let ast = parser.parse(&tokens).into_result().expect("Parse error.");

        assert_yaml_snapshot!(ast);
    }
}
